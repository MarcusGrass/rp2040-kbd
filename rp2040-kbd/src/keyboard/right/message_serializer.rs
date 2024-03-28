use crate::keyboard::split_serial::UartRight;
use crate::keyboard::sync::{ENCODER_MSG_LEN, ENCODER_TAG, MATRIX_STATE_MSG_LEN, MATRIX_STATE_TAG};
use crate::keyboard::{MatrixState, MatrixUpdate, NUM_COLS, NUM_ROWS};
use embedded_io::Write;

const BUF_SIZE: usize = 32;
pub(crate) struct MessageSerializer {
    uart: UartRight,
}

impl MessageSerializer {
    #[inline]
    pub(crate) fn serialize_matrix_state(&mut self, update: &MatrixUpdate) -> bool {
        self.uart
            .inner
            .write_raw(update.as_slice())
            .is_ok()
    }

    pub fn new(uart: UartRight) -> Self {
        Self { uart }
    }
}
