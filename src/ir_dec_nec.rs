//! State module for ir receiver

#![allow(dead_code)]
#![allow(unused_variables)]
use core::cell::Cell;
use core::marker::PhantomData;

const START1_MIN: u32 = 7900; //us
const START1_MAX: u32 = 10100; //us
const START2_MIN: u32 = 3400; //us
const START2_MAX: u32 = 5600; //us

const SHORT_MIN: u32 = 350; //us
const SHORT_MAX: u32 = 750; //us

const LONG_MIN: u32 = 1400; //us
const LONG_MAX: u32 = 1900; //us

#[derive(Clone, Copy, Debug)]
enum IrNecState {
    Reset,
    Start1,
    Start2,
    PulseData,
    Data0,
    Data1,
}


/// State machine for ir receiver
#[derive(Debug)]
#[repr(packed)]
pub struct IrDecoderNec<T> {
    state: Cell<IrNecState>,
    bits: Cell<u32>,
    command: Cell<u32>,
    _marker: PhantomData<T>,
}

///
pub struct IrCode {
    ///
    pub address: u8,
    ///
    pub command: u8,
}

///
pub enum IrDecodeError {
    ///
    NoData,
    ///
    Implausible,
    ///
    Pending,
}

impl<T> IrDecoderNec<T> {
    /// new IR state
    pub const fn new() -> Self {
        IrDecoderNec {
            state: Cell::new(IrNecState::Reset),
            bits: Cell::new(0),
            command: Cell::new(0),
            _marker: PhantomData,
        }
    }


    fn next_state(&self, pulse: u32) {
        let new_state = match self.state.get() {
            IrNecState::Reset if pulse >= START1_MIN && pulse <= START1_MAX => IrNecState::Start1,
            IrNecState::Start1 if pulse >= START2_MIN && pulse <= START2_MAX => IrNecState::Start2,
            IrNecState::Start2 |
            IrNecState::Data0 |
            IrNecState::Data1 if pulse >= SHORT_MIN && pulse <= SHORT_MAX => IrNecState::PulseData,
            IrNecState::PulseData if pulse >= SHORT_MIN && pulse <= SHORT_MAX => IrNecState::Data0,
            IrNecState::PulseData if pulse >= LONG_MIN && pulse <= LONG_MAX => IrNecState::Data1,
            _ => IrNecState::Reset,
        };
        self.state.set(new_state);
    }
    /// reset the state
    fn reset(&self) {
        self.state.set(IrNecState::Reset);
        self.bits.set(0);
        self.command.set(0);
    }

    /// feed
    pub fn feed(&self, period: u32) {
        self.next_state(period);
        match self.state.get() {
            IrNecState::Data0 => {
                self.command.set((self.command.get() << 1) + 0);
                self.bits.set(self.bits.get() + 1);
            }
            IrNecState::Data1 => {

                self.command.set((self.command.get() << 1) + 1);
                self.bits.set(self.bits.get() + 1);
            }
            IrNecState::Reset => {
                self.reset();
            }
            _ => {}
        }
    }

    /// read
    pub fn try_get(&self) -> Result<IrCode, IrDecodeError> {
        if self.bits.get() == 32 {
            let message = self.command.get();
            self.reset();
            let address = ((message >> 24) & 0xFF) as u8;
            let address_inv = ((message >> 16) & 0xFF) as u8;

            if address == (!address_inv) {

                let command = ((message >> 8) & 0xFF) as u8;
                let command_inv = ((message) & 0xFF) as u8;
                if command == (!command_inv) {
                    return Ok(IrCode { address, command });
                }
            }

            return Err(IrDecodeError::Implausible);
        }
        match self.state.get() {
            IrNecState::Reset => Err(IrDecodeError::NoData),
            _ => Err(IrDecodeError::Pending),
        }
    }
}
