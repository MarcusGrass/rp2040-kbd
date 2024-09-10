use embedded_hal::digital::{InputPin, OutputPin};
use rp2040_hal::gpio::bank0::Gpio24;
use rp2040_hal::gpio::{FunctionSio, Pin, PullDown, SioOutput};

pub struct PowerLed {
    state: bool,
    pin: Pin<Gpio24, FunctionSio<SioOutput>, PullDown>,
}

impl PowerLed {
    pub fn new(pin: Pin<Gpio24, FunctionSio<SioOutput>, PullDown>) -> Self {
        let state = matches!(pin.as_input().is_low(), Ok(true));
        Self { state, pin }
    }

    #[inline]
    #[cfg(feature = "serial")]
    pub fn is_on(&self) -> bool {
        self.state
    }

    #[inline]
    pub fn turn_on(&mut self) {
        if !self.state {
            self.set_true();
        }
    }

    #[cold]
    #[inline(never)]
    fn set_true(&mut self) {
        let _ = self.pin.set_low();
        self.state = true;
    }

    #[inline]
    pub fn turn_off(&mut self) {
        if self.state {
            self.set_false();
        }
    }

    #[cold]
    #[inline(never)]
    fn set_false(&mut self) {
        let _ = self.pin.set_high();
        self.state = false;
    }
}
