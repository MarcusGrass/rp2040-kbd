use embedded_io::Read;
use pio_uart::PioSerialError;
use crate::keyboard::{ButtonState, ButtonStateChange, INITIAL_STATE, matrix_ind, MatrixState, NUM_COLS, NUM_ROWS};
use crate::keyboard::split_serial::UartLeft;
use crate::keyboard::sync::{ENCODER_TAG, MATRIX_STATE_TAG, ENCODER_MSG_LEN, MATRIX_STATE_MSG_LEN};

const BUF_SIZE: usize = 512;
pub(crate) struct MessageReceiver {
    uart: UartLeft,
    pub(crate) buf: [u8; BUF_SIZE],
    pub(crate) cursor: usize,
    pub(crate) total_read: usize,
    pub(crate) successful_reads: usize,
    pub(crate) unk_msg: usize,
    pub(crate) bad_matrix: usize,
    pub(crate) good_matrix: usize,
    pub(crate) unk_rollback: usize,
    matrix: MatrixState,
    changes: heapless::Vec<ButtonStateChange, 16>,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub(crate) enum EncoderDirection {
    Clockwise,
    CounterClockwise,
}

impl EncoderDirection {
    fn try_from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Clockwise),
            1 => Some(Self::CounterClockwise),
            _ => None,
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

        Self { uart, buf: [0u8; BUF_SIZE], cursor: 0, changes: heapless::Vec::new(), matrix: INITIAL_STATE, total_read: 0, successful_reads: 0, unk_msg: 0, bad_matrix: 0, good_matrix: 0, unk_rollback: 0 }
    }

    pub(crate) fn try_read(&mut self) -> Option<DeserializedMessage> {
        let res = if let Some(left) = self.buf.get_mut(self.cursor..) {
            self.uart.inner.read(left)
        } else {
            self.cursor = 0;
            return None;
        };
        match res {
            Ok(r) => {
                self.total_read += r;
                if r == 0 {
                    return None;
                }
                self.successful_reads += 1;
                self.cursor += r;
                self.try_message()
            }
            Err(_) => {
                None
            }
        }
    }

    fn try_message(&mut self) -> Option<DeserializedMessage> {
        match self.buf[0] {
            MATRIX_STATE_TAG => {
                if self.cursor < MATRIX_STATE_MSG_LEN {
                    return None;
                }
                let target: [u8; 4] = self.buf[1..5].try_into().unwrap();
                let state: MatrixState = MatrixState::new(target);
                for row_ind in 0..NUM_ROWS {
                    for col_ind in 0..NUM_COLS {
                        let ind = matrix_ind(row_ind, col_ind);
                        let old = self.matrix[ind];
                        let new = state[ind];
                        if old != new {
                            let _ = self.changes.push(ButtonStateChange::new(row_ind as u8, col_ind as u8, new.into()));
                            self.matrix.set(ind, new);
                        }
                    }
                }
                self.good_matrix += 1;
                self.cursor = 0;
                Some(DeserializedMessage::Matrix(&self.matrix))
            }
            ENCODER_TAG => {
                if self.cursor < ENCODER_MSG_LEN {
                    return None;
                }

                if let Some(direction) = EncoderDirection::try_from_byte(self.buf[1]) {
                    self.cursor = 0;
                    return Some(DeserializedMessage::Encoder(direction));
                }
                self.cursor = 0;
                None
            }
            _unk => {
                self.unk_rollback += 1;
                let mut valid_at = None;
                for i in 0..self.cursor {
                    if let Some(next) = self.buf.get(i) {
                        match *next {
                            MATRIX_STATE_TAG | ENCODER_TAG => {
                                valid_at = Some(i);
                                break;
                            },
                            _ => {}
                        }
                    } else {
                        break;
                    }
                }
                if let Some(ind_of_valid) = valid_at {
                    self.buf.copy_within(ind_of_valid..self.cursor, 0);
                    // Move up cursor as much
                    self.cursor -= ind_of_valid;
                    return self.try_message();
                }
                self.cursor = 0;
                self.unk_msg += 1;
                None
            }
        }
    }
}
