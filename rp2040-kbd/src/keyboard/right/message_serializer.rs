use crate::keyboard::split_serial::UartRight;
use rp2040_kbd_lib::matrix::MatrixUpdate;

pub(crate) struct MessageSerializer {
    uart: UartRight,
}

impl MessageSerializer {
    #[inline]
    pub(crate) fn serialize_matrix_state(&mut self, update: MatrixUpdate) {
        self.uart.inner.blocking_write_byte(update.byte());
    }

    pub fn new(uart: UartRight) -> Self {
        Self { uart }
    }
}
