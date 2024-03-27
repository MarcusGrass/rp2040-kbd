pub(crate) mod message_serializer;

use crate::check_col_no_store;
use crate::keyboard::usb_serial::UsbSerial;
use crate::keyboard::{
    matrix_ind, ButtonPin, ButtonState, ButtonStateChange, MatrixState, RowPin, INITIAL_STATE,
    NUM_COLS, NUM_ROWS,
};
use crate::runtime::shared::usb::acquire_usb;
use core::fmt::Write;
use embedded_hal::digital::v2::{InputPin, OutputPin, PinState};
use rp2040_hal::gpio::bank0::{
    Gpio20, Gpio21, Gpio22, Gpio23, Gpio26, Gpio27, Gpio29, Gpio4, Gpio5, Gpio6, Gpio7, Gpio8,
    Gpio9,
};
use rp2040_hal::gpio::{FunctionSio, Pin, PullUp, SioInput};

pub struct RightButtons {
    pub(crate) matrix: MatrixState,
    rows: [RowPin; 5],
    cols: (
        Option<ButtonPin<Gpio22>>,
        Option<ButtonPin<Gpio5>>,
        Option<ButtonPin<Gpio6>>,
        Option<ButtonPin<Gpio7>>,
        Option<ButtonPin<Gpio8>>,
        Option<ButtonPin<Gpio9>>,
    ),
}

impl RightButtons {
    pub fn new(
        rows: (
            ButtonPin<Gpio29>,
            ButtonPin<Gpio4>,
            ButtonPin<Gpio20>,
            ButtonPin<Gpio23>,
            ButtonPin<Gpio21>,
        ),
        cols: (
            Option<ButtonPin<Gpio22>>,
            Option<ButtonPin<Gpio5>>,
            Option<ButtonPin<Gpio6>>,
            Option<ButtonPin<Gpio7>>,
            Option<ButtonPin<Gpio8>>,
            Option<ButtonPin<Gpio9>>,
        ),
    ) -> Self {
        Self {
            matrix: INITIAL_STATE,
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

    pub fn scan_matrix(&mut self) -> bool {
        if check_col_no_store!(self, 0)
            || check_col_no_store!(self, 1)
            || check_col_no_store!(self, 2)
            || check_col_no_store!(self, 3)
            || check_col_no_store!(self, 4)
            || check_col_no_store!(self, 5)
        {
            true
        } else {
            false
        }
    }
}

pub struct RotaryEncoder {
    pin_a: Pin<Gpio26, FunctionSio<SioInput>, PullUp>,
    pin_b: Pin<Gpio27, FunctionSio<SioInput>, PullUp>,
}

impl RotaryEncoder {
    pub fn new(
        pin_a: Pin<Gpio26, FunctionSio<SioInput>, PullUp>,
        pin_b: Pin<Gpio27, FunctionSio<SioInput>, PullUp>,
    ) -> Self {
        Self { pin_a, pin_b }
    }
}
