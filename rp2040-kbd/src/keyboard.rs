//! Common between sides, put everything with the same pinouts and shared hardware
//! code here
pub mod left;
pub mod right;
pub mod oled;
pub mod split_serial;
pub mod usb_serial;
pub mod power_led;
mod sync;

use embedded_hal::digital::v2::{InputPin, OutputPin};
use rp2040_hal::gpio::{DynPinId, FunctionSio, Pin, PinId, PullUp, SioInput};

type RowPin = Pin<DynPinId, FunctionSio<SioInput>, PullUp>;
type ButtonPin<Id> = Pin<Id, FunctionSio<SioInput>, PullUp>;

pub const NUM_COLS: usize = 6;
pub const NUM_ROWS: usize = 5;

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

impl ButtonState {
    #[inline]
    pub(crate) fn try_from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::Depressed),
            1 => Some(Self::Pressed),
            _ => None
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

pub type MatrixState = [[ButtonState; NUM_COLS]; NUM_ROWS];

pub(crate) const INITIAL_STATE: MatrixState = [[ButtonState::Depressed; NUM_COLS]; NUM_ROWS];

#[macro_export]
macro_rules! check_col {
    ($slf: expr, $pt: tt, $m_state: expr, $vec: expr) => {
        {
            let mut col = $slf.cols.$pt.take().unwrap();
            let mut col = col.into_push_pull_output_in_state(PinState::Low);
            for (ind, row) in $slf.rows.iter().enumerate() {
                let state = if matches!(row.is_low(), Ok(true)) {
                    ButtonState::Pressed
                } else {
                    ButtonState::Depressed
                };
                if state != $slf.prev_matrix[ind][$pt] {
                    let _ = $vec.push(ButtonStateChange {
                        row: ind as u8,
                        col: $pt,
                        new_state: state,
                    });
                }
                $m_state[ind][$pt] = state;
            }
            let _ = col.set_high();
            $slf.cols.$pt = Some(col.into_pull_up_input());
        }

    };
}

#[macro_export]
macro_rules! check_col_no_store {
    ($slf: expr, $pt: tt, $m_state: expr) => {
        {
            let mut col = $slf.cols.$pt.take().unwrap();
            let mut col = col.into_push_pull_output_in_state(PinState::Low);
            let mut changed = false;
            for (ind, row) in $slf.rows.iter().enumerate() {
                let state = if matches!(row.is_low(), Ok(true)) {
                    ButtonState::Pressed
                } else {
                    ButtonState::Depressed
                };
                if state != $slf.matrix[ind][$pt] {
                    changed = true;
                }
                $m_state[ind][$pt] = state;
            }
            let _ = col.set_high();
            $slf.cols.$pt = Some(col.into_pull_up_input());
            changed
        }

    };
}