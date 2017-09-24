//! InTimer
#![allow(dead_code)]
#![allow(unused_variables)]
//use core::any::{Any, TypeId};
use core::u16;
use cast::u16;
//use hal;
//use nb::{self, Error};

use stm32f103xx::{TIM1, GPIOA, AFIO, RCC};

/// `hal::InTimer` implementation
pub struct IrCapture<'a, T>(pub &'a T)
where
    T: 'a;

impl<'a, T> Clone for IrCapture<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T> Copy for IrCapture<'a, T> {}

///  edge state
#[derive(Debug, Clone, Copy)]
pub enum LevelState {
    /// rising level
    Rising,
    /// falling level
    Falling,
}

/// Timer state
#[derive(Debug, Clone, Copy)]
pub struct IrCaptureEvent {
    /// state
    pub level: LevelState,
    /// timer1
    pub time1: u32,
    /// timer2
    pub time2: u32,
}

impl<'a> IrCapture<'a, TIM1> {
    /// Initializes the timer with a periodic timeout of `frequency` Hz
    ///
    /// NOTE After initialization, the timer will be in the paused state.
    pub fn init<P>(&self, resolution: P, afio: &AFIO, gpioa: &GPIOA, rcc: &RCC)
    where
        P: Into<::apb2::Ticks>,
    {
        self._init(resolution.into(), afio, gpioa, rcc)
    }

    fn _init(&self, resolution: ::apb2::Ticks, afio: &AFIO, gpioa: &GPIOA, rcc: &RCC) {
        let tim1 = self.0;

        // enable AFIO, TIM1 and GPIOA
        rcc.apb2enr.modify(|_, w| {
            w.tim1en().enabled().afioen().enabled().iopaen().enabled()
        });

        // don't remap TIM1 pins
        afio.mapr.modify(
            |_, w| unsafe { w.tim1_remap().bits(0b00) },
        );

        self.set_resolution(resolution);

        // CH1 = PA8 = floating input
        gpioa.crh.modify(|_, w| {
            w.mode8().input().cnf8().bits(0b01)
        });

        tim1.ccer.modify(|_, w| {
            w.cc1e()
                .clear_bit()
                .cc2e()
                .clear_bit()
                .cc3e()
                .clear_bit()
                .cc4e()
                .clear_bit()
        });

        // Select the active input for TIMx_CCR1:
        // write the CC1S bits to 01 in the TIMx_CCMR1 register (TI1 selected).
        //+ 		tim1.ccmr1_output.write(|w| unsafe { w.bits(0b01 << 0) });
        // Select the active input for TIMx_CCR2:
        // write the CC2S bits to 10 in the TIMx_CCMR1 register (TI1 selected).
        //+			tim1.ccmr1_output.write(|w| unsafe { w.bits(0b10 << 8) });

        // configure CC{1,2,3,4} as input and wire it to TI{1,2,3,4}
        // apply the heaviest filter
        tim1.ccmr1_output.write(|w| unsafe {
            w.bits((0b1111 << 12) | (0b10 << 8) | (0b1111 << 4) | (0b01 << 0))
        });

        // enable capture on falling/rising edge
        // Select the active polarity for TI1FP1 (used both for capture in TIMx_CCR1 and counter
        //  clear): write the CC1P bit to ‘0’ (active on rising edge).
        // Select the active polarity for TI1FP2 (used for capture in TIMx_CCR2): write the CC2P
        //  bit to ‘1’ (active on falling edge).
        tim1.ccer.modify(
            |_, w| w.cc1p().set_bit().cc2p().clear_bit(),
        );

        tim1.arr.write(|w| w.arr().bits(u16::MAX));

        //Select the valid trigger input: write the TS bits to 101 in the TIMx_SMCR register
        //(TI1FP1 selected).
        tim1.smcr.write(|w| unsafe { w.bits(0b101 << 4) });


        //Configure the slave mode controller in reset mode: write the SMS bits to 100 in the
        //TIMx_SMCR register.

        tim1.smcr.write(|w| unsafe { w.bits(0b100 << 0) });


        //Enable capture/compare event interrupt
        //Enable update event interrupt
        tim1.cr1.write(
            |w| w.dir().up().opm().continuous().cen().enabled(),
        );
        tim1.dier.modify(
            |_, w| w.cc1ie().set_bit().cc2ie().set_bit(),
        );
        tim1.ccer.modify(|_, w| w.cc1e().set_bit().cc2e().set_bit());
        let _s1 = tim1.sr.read().cc3if().bit();
        let _s2 = tim1.sr.read().cc4if().bit();
    }

    ///
    pub fn set_resolution(&self, resolution: ::apb2::Ticks) {
        let psc = u16(resolution.0.checked_sub(1).expect("impossible resolution")).unwrap();

        self.0.psc.write(|w| w.psc().bits(psc));
    }

    ///
    pub fn get_event(&self) -> Option<IrCaptureEvent> {
        let tim1 = self.0;
        let _s = tim1.sr.read().bits();
        let s1 = tim1.sr.read().cc1if().bit();
        let time1 = tim1.ccr1.read().bits();
        let s2 = tim1.sr.read().cc2if().bit();
        let time2 = tim1.ccr2.read().bits();
        if !s1 && !s2 {
            None
        } else {
            Some(IrCaptureEvent {
                time1,
                time2,
                level: if s1 {
                    LevelState::Falling
                } else {
                    LevelState::Rising
                },
            })
        }
    }
}
