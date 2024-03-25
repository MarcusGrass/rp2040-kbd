use crate::keyboard::{ButtonStateChange, MatrixState, NUM_COLS, NUM_ROWS};
pub const MATRIX_STATE_TAG: u8 = u8::MAX;
pub const MATRIX_STATE_MSG_LEN: usize = NUM_ROWS * NUM_COLS + 1;
pub const ENCODER_TAG: u8 = u8::MAX - 1;

pub const ENCODER_MSG_LEN: usize = 2;
