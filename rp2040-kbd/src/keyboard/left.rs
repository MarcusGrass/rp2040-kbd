pub(crate) mod message_receiver;

use crate::keyboard::ButtonPin;
use rp2040_hal::gpio::bank0::{
    Gpio20, Gpio21, Gpio22, Gpio23, Gpio26, Gpio27, Gpio29, Gpio6, Gpio7, Gpio8, Gpio9,
};

pub const ROW0: u32 = 1 << 29;
pub const ROW1: u32 = 1 << 27;
pub const ROW2: u32 = 1 << 6;
pub const ROW3: u32 = 1 << 7;
pub const ROW4: u32 = 1 << 8;
pub const ROW_MASK: u32 = ROW0 | ROW1 | ROW2 | ROW3 | ROW4;

pub struct LeftButtons {
    pub _rows: (
        ButtonPin<Gpio29>,
        ButtonPin<Gpio27>,
        ButtonPin<Gpio6>,
        ButtonPin<Gpio7>,
        ButtonPin<Gpio8>,
    ),
    pub cols: (
        Option<ButtonPin<Gpio9>>,
        Option<ButtonPin<Gpio26>>,
        Option<ButtonPin<Gpio22>>,
        Option<ButtonPin<Gpio20>>,
        Option<ButtonPin<Gpio23>>,
        Option<ButtonPin<Gpio21>>,
    ),
}

impl LeftButtons {
    pub fn new(
        rows: (
            ButtonPin<Gpio29>,
            ButtonPin<Gpio27>,
            ButtonPin<Gpio6>,
            ButtonPin<Gpio7>,
            ButtonPin<Gpio8>,
        ),
        cols: (
            Option<ButtonPin<Gpio9>>,
            Option<ButtonPin<Gpio26>>,
            Option<ButtonPin<Gpio22>>,
            Option<ButtonPin<Gpio20>>,
            Option<ButtonPin<Gpio23>>,
            Option<ButtonPin<Gpio21>>,
        ),
    ) -> Self {
        Self { _rows: rows, cols }
    }
}
