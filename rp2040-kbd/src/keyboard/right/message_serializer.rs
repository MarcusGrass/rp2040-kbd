use embedded_io::Write;
use crate::keyboard::{MatrixState, NUM_COLS, NUM_ROWS};
use crate::keyboard::split_serial::UartRight;
use crate::keyboard::sync::{ENCODER_MSG_LEN, ENCODER_TAG, MATRIX_STATE_MSG_LEN, MATRIX_STATE_TAG};

const BUF_SIZE: usize = 32;
pub(crate) struct MessageSerializer {
    uart: UartRight,
}

impl MessageSerializer {

    pub(crate) fn serialize_matrix_state(&mut self, state: &MatrixState) -> bool {
        self.uart.inner.write_raw(&[MATRIX_STATE_TAG, state.data[0], state.data[1], state.data[2], state.data[3]]).is_ok()
    }

    pub(crate) fn serialize_rotary_encoder(&mut self, clockwise: bool) -> bool {
        self.uart.inner.write_raw(&[ENCODER_TAG, clockwise as u8]).is_ok()
    }
    pub fn new(uart: UartRight) -> Self {
        Self { uart }
    }
}

