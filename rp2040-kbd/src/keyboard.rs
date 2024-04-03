//! Common between sides, put everything with the same pinouts and shared hardware
//! code here
#[cfg(feature = "left")]
pub mod left;
pub mod oled;
pub mod power_led;
#[cfg(feature = "right")]
pub mod right;
pub mod split_serial;
#[cfg(feature = "serial")]
pub mod usb_serial;

use bitvec::array::BitArray;
use bitvec::order::Lsb0;
use rp2040_hal::gpio::{DynPinId, FunctionSio, Pin, PullUp, SioInput};

type RowPin = Pin<DynPinId, FunctionSio<SioInput>, PullUp>;
type ButtonPin<Id> = Pin<Id, FunctionSio<SioInput>, PullUp>;

#[cfg(any(feature = "right", feature = "serial"))]
pub const NUM_COLS: usize = 6;
#[cfg(feature = "right")]
pub const NUM_ROWS: usize = 5;

#[inline]
#[cfg(feature = "right")]
pub const fn matrix_ind(row_ind: usize, col_ind: usize) -> usize {
    row_ind * NUM_COLS + col_ind
}

#[cfg(all(feature = "serial", feature = "left"))]
pub const fn matrix_ind_to_row_col(matrix_ind: usize) -> (usize, usize) {
    (matrix_ind / NUM_COLS, matrix_ind % NUM_COLS)
}

#[cfg(feature = "right")]
pub type MatrixState = BitArray<[u8; 4], Lsb0>;

#[cfg(feature = "right")]
pub(crate) const INITIAL_STATE: MatrixState = BitArray::ZERO;

#[derive(Debug, Copy, Clone)]
pub(crate) struct MatrixUpdate(BitArray<[u8; 1], Lsb0>);

#[cfg(feature = "left")]
#[derive(Debug, Copy, Clone)]
pub enum MatrixChange {
    Key(u8, bool),
    Encoder(bool),
}

impl MatrixUpdate {
    #[inline]
    #[cfg(feature = "right")]
    pub fn new_keypress(matrix_ind: u8, state: bool) -> Self {
        let mut inner = BitArray::new([matrix_ind; 1]);
        inner.set(5, state);
        Self(inner)
    }

    #[cfg(feature = "right")]
    pub fn new_encoder_rotation(clockwise: bool) -> Self {
        let mut inner = BitArray::new([0u8; 1]);
        inner.set(6, true);
        inner.set(7, clockwise);
        Self(inner)
    }

    #[inline]
    #[cfg(feature = "left")]
    pub fn from_byte(byte: u8) -> Self {
        Self(BitArray::new([byte; 1]))
    }

    #[inline]
    #[cfg(feature = "left")]
    fn matrix_ind(self) -> u8 {
        self.0.data[0] & 0b0001_1111
    }

    #[inline]
    #[cfg(feature = "left")]
    pub fn matrix_change(self) -> MatrixChange {
        if let Some(enc) = self.encoder_state() {
            MatrixChange::Encoder(enc)
        } else {
            MatrixChange::Key(self.matrix_ind(), self.0[5])
        }
    }

    #[inline]
    #[cfg(feature = "left")]
    pub fn encoder_state(self) -> Option<bool> {
        if self.0[6] {
            Some(self.0[7])
        } else {
            None
        }
    }

    #[inline]
    #[cfg(feature = "right")]
    pub fn as_slice(&self) -> &[u8] {
        &self.0.data
    }
}
