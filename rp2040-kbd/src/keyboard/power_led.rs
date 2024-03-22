use embedded_hal::digital::v2::{InputPin, OutputPin};
use rp2040_hal::gpio::bank0::Gpio24;
use rp2040_hal::gpio::{FunctionSio, Pin, PullDown, SioOutput};

pub struct PowerLed {
    pin: Pin<Gpio24, FunctionSio<SioOutput>, PullDown>,
}

impl PowerLed {
    pub fn new(pin: Pin<Gpio24, FunctionSio<SioOutput>, PullDown>) -> Self {
        Self { pin }
    }

    #[inline]
    pub fn is_on(&self) -> bool {
        matches!(self.pin.is_high(), Ok(true))
    }

    #[inline]
    pub fn turn_on(&mut self) {
        let _ = self.pin.set_high();
    }

    #[inline]
    pub fn turn_off(&mut self) {
        let _ = self.pin.set_low();
    }
}