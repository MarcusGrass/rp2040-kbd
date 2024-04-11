pub(crate) mod message_receiver;

use crate::keyboard::{ButtonPin, RowPin};
use embedded_hal::digital::InputPin;
use rp2040_hal::gpio::bank0::{
    Gpio20, Gpio21, Gpio22, Gpio23, Gpio26, Gpio27, Gpio29, Gpio6, Gpio7, Gpio8, Gpio9,
};
use rp2040_kbd_lib::matrix::{RowIndex, NUM_ROWS};

pub struct LeftButtons {
    pub rows: [RowPin; NUM_ROWS as usize],
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
        Self {
            rows: [
                rows.0.into_dyn_pin(),
                rows.1.into_dyn_pin(),
                rows.2.into_dyn_pin(),
                rows.3.into_dyn_pin(),
                rows.4.into_dyn_pin(),
            ],
            cols,
        }
    }

    #[inline]
    pub fn row_pin_is_low(&mut self, row_index: RowIndex) -> bool {
        unsafe {
            // Safety: Index in range by type, `is_low` is infallible
            self.rows
                .get_unchecked_mut(row_index.index())
                .is_low()
                .unwrap_unchecked()
        }
    }
}
