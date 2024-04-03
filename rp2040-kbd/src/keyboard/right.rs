pub(crate) mod message_serializer;

use crate::check_col_push_evt;
use crate::keyboard::right::message_serializer::MessageSerializer;
use crate::keyboard::usb_serial::UsbSerial;
use crate::keyboard::{
    matrix_ind, ButtonPin, ButtonState, ButtonStateChange, MatrixState, MatrixUpdate, RowPin,
    INITIAL_STATE, NUM_COLS, NUM_ROWS,
};
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
    encoder: RotaryEncoder,
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
        rotary_encoder: RotaryEncoder,
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
            encoder: rotary_encoder,
        }
    }

    pub fn scan_matrix(&mut self, serializer: &mut MessageSerializer) -> bool {
        let col0_change = check_col_push_evt!(self, 0, serializer);
        let col1_change = check_col_push_evt!(self, 1, serializer);
        let col2_change = check_col_push_evt!(self, 2, serializer);
        let col3_change = check_col_push_evt!(self, 3, serializer);
        let col4_change = check_col_push_evt!(self, 4, serializer);
        let col5_change = check_col_push_evt!(self, 5, serializer);
        col0_change || col1_change || col2_change || col3_change || col4_change || col5_change
    }

    #[inline]
    pub fn scan_encoder(&mut self, serializer: &mut MessageSerializer) -> bool {
        if let Some(dir) = self.encoder.scan_debounced() {
            #[cfg(feature = "serial")]
            {
                let _ = crate::runtime::shared::usb::acquire_usb().write_fmt(format_args!(
                    "Encoder clockwise: Pos: {:?} clockwise={:?}\r\n",
                    self.encoder.last_position, dir
                ));
            }
            serializer.serialize_matrix_state(&MatrixUpdate::new_encoder_rotation(dir));
            true
        } else {
            false
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum RotaryPosition {
    North,
    East,
    South,
    West,
}

impl RotaryPosition {
    fn from_state(a: bool, b: bool) -> Self {
        match (a, b) {
            (true, true) => RotaryPosition::South,
            (false, true) => RotaryPosition::West,
            (false, false) => RotaryPosition::North,
            (true, false) => RotaryPosition::East,
        }
    }
}

pub struct RotaryEncoder {
    pin_a: Pin<Gpio26, FunctionSio<SioInput>, PullUp>,
    pin_b: Pin<Gpio27, FunctionSio<SioInput>, PullUp>,
    last_position: Option<RotaryPosition>,
    last_clockwise: Option<bool>,
    cached: Option<bool>,
}

impl RotaryEncoder {
    pub fn new(
        pin_a: Pin<Gpio26, FunctionSio<SioInput>, PullUp>,
        pin_b: Pin<Gpio27, FunctionSio<SioInput>, PullUp>,
    ) -> Self {
        Self {
            pin_a,
            pin_b,
            last_position: None,
            last_clockwise: None,
            cached: None,
        }
    }

    #[inline]
    fn read_position(&self) -> RotaryPosition {
        let new_pin_a_state = matches!(self.pin_a.is_high(), Ok(true));
        let new_pin_b_state = matches!(self.pin_b.is_high(), Ok(true));
        RotaryPosition::from_state(new_pin_a_state, new_pin_b_state)
    }

    // Dirty, but works for debouncing the encoder
    #[inline]
    pub fn scan_debounced(&mut self) -> Option<bool> {
        let current = self.read_position();
        let Some(old) = self.last_position else {
            self.last_position = Some(current);
            return None;
        };
        let dir = match (old, current) {
            (RotaryPosition::North, RotaryPosition::East)
            | (RotaryPosition::East, RotaryPosition::South)
            | (RotaryPosition::South, RotaryPosition::West)
            | (RotaryPosition::West, RotaryPosition::North) => true,
            (RotaryPosition::North, RotaryPosition::West)
            | (RotaryPosition::West, RotaryPosition::South)
            | (RotaryPosition::South, RotaryPosition::East)
            | (RotaryPosition::East, RotaryPosition::North) => false,
            (_, _) => {
                self.last_position = Some(current);
                return None;
            }
        };
        self.last_position = Some(current);
        let Some(last) = self.last_clockwise else {
            self.last_clockwise = Some(dir);
            return None;
        };
        self.last_clockwise = Some(dir);
        if last == dir {
            if let Some(prev) = self.cached {
                if prev == last {
                    return self.cached.take();
                }
            }
            self.cached = Some(dir);
        }
        None
    }
}
