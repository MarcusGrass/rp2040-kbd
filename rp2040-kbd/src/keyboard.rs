//! Common between sides, put everything with the same pinouts and shared hardware
//! code here
pub mod debounce;
#[cfg(feature = "left")]
pub mod left;
pub mod oled;
pub mod power_led;
#[cfg(feature = "right")]
pub mod right;
pub mod split_serial;
#[cfg(feature = "serial")]
pub mod usb_serial;

use rp2040_hal::gpio::{DynPinId, FunctionSio, Pin, PullUp, SioInput};

type RowPin = Pin<DynPinId, FunctionSio<SioInput>, PullUp>;
pub type ButtonPin<Id> = Pin<Id, FunctionSio<SioInput>, PullUp>;

#[cfg(all(feature = "serial", feature = "left"))]
pub const fn matrix_ind_to_row_col(matrix_ind: u8) -> (u8, u8) {
    (
        matrix_ind / rp2040_kbd_lib::matrix::NUM_COLS,
        matrix_ind % rp2040_kbd_lib::matrix::NUM_COLS,
    )
}

#[cfg(feature = "right")]
pub(crate) const INITIAL_STATE: rp2040_kbd_lib::matrix::MatrixState =
    rp2040_kbd_lib::matrix::MatrixState::empty();
