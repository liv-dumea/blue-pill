#![deny(unsafe_code)]
#![deny(warnings)]
#![feature(proc_macro)]
#![feature(const_fn)]
#![no_std]

extern crate blue_pill;
extern crate nb;
extern crate cortex_m_rtfm as rtfm;
use blue_pill::Serial;
use blue_pill::IrCapture;
use blue_pill::led::{self, PC13};
use blue_pill::prelude::*;
use blue_pill::serial::Event;
use blue_pill::time::Hertz;
use blue_pill::time::Microseconds;
use blue_pill::ir_dec_nec::{IrDecoderNec, IrDecodeError};
use blue_pill::stm32f103xx::TIM1;
use rtfm::{app, Threshold};
use core::u16;


const BAUD_RATE: Hertz = Hertz(115_200);
const BAUD_RATE_BT: Hertz = Hertz(9600);
const RESOLUTION: u32 = 1; // us


app! {
    device: blue_pill::stm32f103xx,
    resources: {
        static ON: bool = false;
		static IR_STATE: IrDecoderNec<TIM1> = IrDecoderNec::<TIM1>::new();
    },

    tasks: {

		TIM1_CC: {
			path: on_capture,
			resources: [TIM1, ON, IR_STATE, USART2],
		},
        USART3: {
            path: on_recv_serial,
            resources: [USART3, USART2],
        },
        USART2: {
            path: on_recv_bt,
            resources: [USART3, USART2],
        },
    },
}

fn init(p: init::Peripherals, _r: init::Resources) {

    led::init(p.GPIOC, p.RCC);

    let serial = Serial(p.USART3);
    serial.init(BAUD_RATE.invert(), p.AFIO, None, p.GPIOB, p.RCC);
    serial.listen(Event::Rxne);

    let bt = Serial(p.USART2);
    bt.init(BAUD_RATE_BT.invert(), p.AFIO, None, p.GPIOA, p.RCC);
    bt.listen(Event::Rxne);

    let capture = IrCapture(p.TIM1);
    capture.init(Microseconds(RESOLUTION), p.AFIO, p.GPIOA, p.RCC);

}

fn idle() -> ! {
    loop {
        rtfm::wfi();
    }
}

fn nec_code_char(code: u8) -> char {
    match code {
        0xA2 => 'p',
        0x62 => 'c',
        0xE2 => 'n',

        0x22 => '<', 
        0x02 => '>',
        0xc2 => '#',

        0xE0 => '-',
        0xA8 => '+', 
        0x90 => '=',

        0x68 => '0',
        0x98 => 'h',
        0xb0 => 't',

        0x30 => '1',
        0x18 => '2',
        0x7A => '3',

        0x10 => '4',
        0x38 => '5',
        0x5A => '6',

        0x42 => '7',
        0x4A => '8',
        0x52 => '9',
        _ => 'x',
    }
}

fn on_capture(_t: &mut Threshold, r: TIM1_CC::Resources) {

    let capture = IrCapture(&**r.TIM1);
    let ir_event = capture.get_event();

    if let Some(res) = ir_event {

        let diff = if res.time2 > res.time1 {
            res.time2 - res.time1
        } else {
            res.time1 - res.time2
        };
        let diff = if diff > (u16::MAX as u32) / 2 {
            u16::MAX as u32 - diff
        } else {
            diff
        };

        let state = &**r.IR_STATE;
        state.feed(diff);

        let result = state.try_get();
        match result {
            Ok(data) => {
                PC13.off();
                let bt = Serial(&**r.USART2);
                while let Err(nb::Error::WouldBlock) = bt.write('c' as u8) {}
                while let Err(nb::Error::WouldBlock) = bt.write(nec_code_char(data.command) as u8) {
                }
                while let Err(nb::Error::WouldBlock) = bt.write('\n' as u8) {}
            }
            Err(IrDecodeError::Pending) => {
                **r.ON = !**r.ON;
                if **r.ON {
                    PC13.off();
                } else {
                    PC13.on();
                }

            }
            _ => {
                PC13.off();
            }
        }
    }

}



fn on_recv_serial(_t: &mut Threshold, r: USART3::Resources) {
    let serial = Serial(&**r.USART3);
    let bt = Serial(&**r.USART2);

    let byte = serial.read().unwrap();
    serial.write(byte).unwrap();
    bt.write(byte).unwrap();
}

fn on_recv_bt(_t: &mut Threshold, r: USART2::Resources) {
    let serial = Serial(&**r.USART3);
    let bt = Serial(&**r.USART2);

    let byte = bt.read().unwrap();
    //bt.write(byte).unwrap();
    serial.write(byte).unwrap();
}
