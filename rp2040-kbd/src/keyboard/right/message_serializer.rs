use embedded_io::Write;
use crate::keyboard::{MatrixState, NUM_COLS, NUM_ROWS};
use crate::keyboard::split_serial::UartRight;
use crate::keyboard::sync::{ENCODER_TAG, MATRIX_STATE_TAG};

pub(crate) struct MessageSerializer {
    uart: UartRight,
    buf: [u8; 128],
    cursor: usize,
    needs_flush: bool,
}

impl MessageSerializer {
    const MATRIX_MSG_LEN: usize = NUM_ROWS * NUM_ROWS + 1;

    #[inline]
    pub(crate) fn clear(&mut self) {
        self.cursor = 0;
    }

    #[inline]
    fn remaining(&self) -> usize {
        self.buf.len() - self.cursor
    }

    pub(crate) fn pump(&mut self) -> bool {
        // Exit if no write necessary
        if self.cursor == 0 {
            return true;
        }
        if let Ok(written) = self.uart.inner.write(&self.buf[..self.cursor]) {
            if written == self.cursor {
                self.cursor = 0;
            } else if written > 0 {
                self.buf.copy_within(written..self.cursor, 0);
                self.cursor -= written;
            }
            true
        } else {
            false
        }
    }

    pub(crate) fn serialize_matrix_state(&mut self, state: &MatrixState) -> bool {
        if self.remaining() < Self::MATRIX_MSG_LEN {
            return false;
        }
        self.buf[self.cursor] = MATRIX_STATE_TAG;
        self.cursor += 1;
        self.buf[self.cursor..self.cursor + 4].copy_from_slice(&state.data);
        self.cursor += 4;
        true
    }

    pub(crate) fn serialize_rotary_encoder(&mut self, clockwise: bool) -> bool {
        if self.remaining() < 2 {
            return false;
        }
        self.buf[self.cursor] = ENCODER_TAG;
        self.buf[self.cursor + 1] = clockwise as u8;
        self.cursor += 2;
        true
    }
    pub fn new(uart: UartRight) -> Self {
        Self { uart, buf: [0u8; 128], cursor: 0, needs_flush: false }
    }
}

