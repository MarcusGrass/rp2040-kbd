use crate::keyboard::split_serial::UartLeft;
use rp2040_kbd_lib::matrix::MatrixUpdate;

pub(crate) struct MessageReceiver {
    pub(crate) uart: UartLeft,
}

impl MessageReceiver {
    pub fn new(uart: UartLeft) -> Self {
        Self { uart }
    }

    #[inline]
    pub(crate) fn try_read(&mut self) -> Option<MatrixUpdate> {
        self.uart.inner.read_one().and_then(MatrixUpdate::from_byte)
    }
}
