use crate::keyboard::split_serial::UartRight;
use crate::keyboard::MatrixUpdate;

pub(crate) struct MessageSerializer {
    uart: UartRight,
}

impl MessageSerializer {
    #[inline]
    pub(crate) fn serialize_matrix_state(&mut self, update: &MatrixUpdate) -> bool {
        self.uart.inner.write_raw(update.as_slice()).is_ok()
    }

    pub fn new(uart: UartRight) -> Self {
        Self { uart }
    }
}
