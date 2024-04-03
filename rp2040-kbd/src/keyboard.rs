//! Common between sides, put everything with the same pinouts and shared hardware
//! code here
#[cfg(feature = "left")]
pub mod left;
pub mod oled;
pub mod power_led;
#[cfg(feature = "right")]
pub mod right;
pub mod split_serial;
pub mod usb_serial;

use bitvec::array::BitArray;
use bitvec::order::Lsb0;
use core::fmt::Write;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use rp2040_hal::gpio::{DynPinId, FunctionSio, Pin, PinId, PullUp, SioInput};
use rp2040_hal::rom_data::reset_to_usb_boot;

type RowPin = Pin<DynPinId, FunctionSio<SioInput>, PullUp>;
type ButtonPin<Id> = Pin<Id, FunctionSio<SioInput>, PullUp>;

pub const NUM_COLS: usize = 6;
pub const NUM_ROWS: usize = 5;

#[inline]
pub const fn matrix_ind(row_ind: usize, col_ind: usize) -> usize {
    row_ind * NUM_COLS + col_ind
}

pub const fn matrix_ind_to_row_col(matrix_ind: usize) -> (usize, usize) {
    (matrix_ind / NUM_COLS, matrix_ind % NUM_COLS)
}
#[derive(Debug, Copy, Clone)]
pub enum KeyboardRow {
    One,
    Two,
    Three,
    Four,
    Five,
}

#[derive(Debug, Copy, Clone)]
pub enum KeyboardCol {
    One,
    Two,
    Three,
    Four,
    Five,
    Size,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ButtonState {
    Depressed = 0,
    Pressed = 1,
}

impl From<bool> for ButtonState {
    #[inline]
    fn from(value: bool) -> Self {
        match value {
            true => Self::Pressed,
            false => Self::Depressed,
        }
    }
}

impl ButtonState {
    #[inline]
    pub(crate) fn try_from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Depressed),
            1 => Some(Self::Pressed),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn into_bool(self) -> bool {
        match self {
            ButtonState::Depressed => false,
            ButtonState::Pressed => true,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ButtonStateChange {
    row: u8,
    col: u8,
    new_state: ButtonState,
}

impl ButtonStateChange {
    pub fn new(row: u8, col: u8, new_state: ButtonState) -> Self {
        Self {
            row,
            col,
            new_state,
        }
    }
}

pub type MatrixState = BitArray<[u8; 4], Lsb0>;

pub(crate) const INITIAL_STATE: MatrixState = BitArray::ZERO;

#[derive(Debug, Copy, Clone)]
pub(crate) struct MatrixUpdate(BitArray<[u8; 1], Lsb0>);

#[derive(Debug, Copy, Clone)]
pub enum MatrixChange {
    Key(u8, bool),
    Encoder(bool),
}

impl MatrixUpdate {
    #[inline]
    pub fn new_keypress(matrix_ind: u8, state: bool) -> Self {
        let mut inner = BitArray::new([matrix_ind; 1]);
        inner.set(5, state);
        Self(inner)
    }

    pub fn new_encoder_rotation(clockwise: bool) -> Self {
        let mut inner = BitArray::new([0u8; 1]);
        inner.set(6, true);
        inner.set(7, clockwise);
        Self(inner)
    }

    #[inline]
    pub fn from_byte(byte: u8) -> Self {
        Self(BitArray::new([byte; 1]))
    }

    #[inline]
    fn matrix_ind(self) -> u8 {
        self.0.data[0] & 0b0001_1111
    }

    #[inline]
    pub fn matrix_change(self) -> MatrixChange {
        if let Some(enc) = self.encoder_state() {
            MatrixChange::Encoder(enc)
        } else {
            MatrixChange::Key(self.matrix_ind(), self.0[5])
        }
    }

    #[inline]
    pub fn encoder_state(self) -> Option<bool> {
        if self.0[6] {
            Some(self.0[7])
        } else {
            None
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.0.data
    }
}
