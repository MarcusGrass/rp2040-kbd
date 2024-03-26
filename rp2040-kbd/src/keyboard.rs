//! Common between sides, put everything with the same pinouts and shared hardware
//! code here
pub mod left;
pub mod right;
pub mod oled;
pub mod split_serial;
pub mod usb_serial;
pub mod power_led;
mod sync;
mod layer;

use core::fmt::Write;
use bitvec::array::BitArray;
use bitvec::order::Lsb0;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use rp2040_hal::gpio::{DynPinId, FunctionSio, Pin, PinId, PullUp, SioInput};
use crate::runtime::right::shared::usb_serial::acquire_usb;

type RowPin = Pin<DynPinId, FunctionSio<SioInput>, PullUp>;
type ButtonPin<Id> = Pin<Id, FunctionSio<SioInput>, PullUp>;

pub const NUM_COLS: usize = 6;
pub const NUM_ROWS: usize = 5;

#[inline]
pub const fn matrix_ind(row_ind: usize, col_ind: usize) -> usize {
    row_ind * NUM_COLS + col_ind
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
            _ => None
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
        Self { row, col, new_state }
    }
}

pub type MatrixState = BitArray<[u8; 4], Lsb0>;

pub(crate) const INITIAL_STATE: MatrixState = BitArray::ZERO;

#[macro_export]
macro_rules! check_col_no_store {
    ($slf: expr, $pt: tt) => {
        {
            let mut col = $slf.cols.$pt.take().unwrap();
            let mut col = col.into_push_pull_output_in_state(PinState::Low);
            // Todo: Remove this redundant wait
            let mut changed = false;
            for row_ind in 0..NUM_ROWS {
                let ind = matrix_ind(row_ind, $pt);
                let state = matches!($slf.rows[row_ind].is_low(), Ok(true));
                if state != $slf.matrix[ind] {
                    let _ = acquire_usb().write_fmt(format_args!("R{}, C{} -> {}\r\n", row_ind, $pt, state as u8));
                    changed = true;
                }
                $slf.matrix.set(ind, state);
            }
            let _ = col.set_high();
            $slf.cols.$pt = Some(col.into_pull_up_input());
            changed
        }

    };
}