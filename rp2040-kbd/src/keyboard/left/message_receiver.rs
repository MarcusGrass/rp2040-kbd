use crate::keyboard::split_serial::UartLeft;
use crate::keyboard::sync::{ENCODER_MSG_LEN, ENCODER_TAG, MATRIX_STATE_MSG_LEN, MATRIX_STATE_TAG};
use crate::keyboard::{matrix_ind, ButtonState, ButtonStateChange, MatrixState, INITIAL_STATE, NUM_COLS, NUM_ROWS, MatrixUpdate};
use embedded_io::Read;
use pio_uart::PioSerialError;

const BUF_SIZE: usize = 16;
pub(crate) struct MessageReceiver {
    pub(crate) uart: UartLeft,
    pub(crate) buf: [u8; BUF_SIZE],
    pub(crate) cursor: usize,
    pub(crate) total_read: u16,
    pub(crate) successful_reads: u16,
    pub(crate) unk_msg: u16,
    pub(crate) bad_matrix: u16,
    pub(crate) good_matrix: u16,
    pub(crate) unk_rollback: u16,
    matrix: MatrixState,
    changes: heapless::Vec<ButtonStateChange, 16>,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub(crate) enum EncoderDirection {
    Clockwise = 0,
    CounterClockwise = 1,
}

impl EncoderDirection {
    fn try_from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Clockwise),
            1 => Some(Self::CounterClockwise),
            _ => None,
        }
    }

    #[inline]
    pub fn into_bool(self) -> bool {
        unsafe {
            core::mem::transmute(self)
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum DeserializedMessage<'a> {
    Matrix(&'a MatrixState),
    Encoder(EncoderDirection),
}
impl MessageReceiver {
    pub fn new(uart: UartLeft) -> Self {
        Self {
            uart,
            buf: [0u8; BUF_SIZE],
            cursor: 0,
            changes: heapless::Vec::new(),
            matrix: INITIAL_STATE,
            total_read: 0,
            successful_reads: 0,
            unk_msg: 0,
            bad_matrix: 0,
            good_matrix: 0,
            unk_rollback: 0,
        }
    }

    #[inline]
    pub(crate) fn try_read(&mut self) -> Option<MatrixUpdate> {
        self.uart.inner.read_one()
            .map(MatrixUpdate::from_byte)
    }

}
