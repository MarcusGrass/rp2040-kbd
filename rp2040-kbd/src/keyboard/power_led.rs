use embedded_hal::digital::{InputPin, OutputPin};
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
    #[allow(dead_code)]
    pub fn is_on(&self) -> bool {
        matches!(self.pin.as_input().is_high(), Ok(true))
    }

    #[inline]
    #[allow(dead_code)]
    pub fn turn_on(&mut self) {
        let _ = self.pin.set_high();
    }

    #[inline]
    #[allow(dead_code)]
    pub fn turn_off(&mut self) {
        let _ = self.pin.set_low();
    }
}
