use elite_pi::pac::{PIO0, PIO1, UART0};
use embedded_hal::digital::v2::{InputPin, OutputPin, PinState};
use embedded_hal::timer::CountDown;
use embedded_io::{Read, Write};
use nb::block;
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_hal::gpio::bank0::{Gpio0, Gpio1, Gpio2};
use rp2040_hal::gpio::{FunctionSio, FunctionUart, Pin, PullBusKeep, PullDown, PullUp, SioInput, SioOutput};
use rp2040_hal::pio::{Running, SM0};
use rp2040_hal::Timer;
use rp2040_hal::uart::{Enabled, UartPeripheral};

#[derive(Debug, Copy, Clone)]
pub enum SplitSerialMessage {
    Ping,
    Pong
}


pub struct SplitSerial {
    current: [u8; 16],
    current_offset: u8,
    pin: Pin<Gpio1, FunctionSio<SioOutput>, PullBusKeep>,
    timer: Timer,
}

pub fn serial_delay(timer: &Timer) {
    let mut cd = timer.count_down();
    cd.start(MicrosDurationU64::micros(16));
    let _ = block!(cd.wait());
}



pub struct UartLeft {
    pub(crate) inner: pio_uart::PioUart<Gpio0, Gpio1, PIO0, Running>
}

impl UartLeft {
    pub fn new(inner: pio_uart::PioUart<Gpio0, Gpio1, PIO0, Running>) -> Self {
        Self { inner }
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> bool {
        if self.inner.flush().is_err() {
            return false;
        }
        let mut b = buf;
        let mut offset = 0;
        loop {
            match self.inner.read(&mut b[offset..]) {
                Ok(0) => {}
                Ok(r) => {
                    offset += r;
                    if offset >= b.len() {
                        return true;
                    }
                }
                Err(e) => {
                    return false;
                }
            }
        }
    }

    pub fn write_all(&mut self, mut msg: &[u8]) -> bool {
        let mut written = 0;
        loop {
            if let Ok(w) = self.inner.write(&msg[written..]) {
                written += w;
                if written == msg.len() {
                    break self.inner.flush().is_ok()
                }
            } else {
                break false
            }
        }
    }
}

pub struct UartRight {
    pub(crate) inner: pio_uart::PioUart<Gpio1, Gpio0, PIO0, Running>
}

impl UartRight {
    pub fn new(inner: pio_uart::PioUart<Gpio1, Gpio0, PIO0, Running>) -> Self {
        Self { inner }
    }

    pub fn write_all(&mut self, mut msg: &[u8]) -> bool {
        let mut written = 0;
        loop {
            if let Ok(w) = self.inner.write(&msg[written..]) {
                written += w;
                if written == msg.len() {
                    break self.inner.flush().is_ok()
                }
            } else {
                break false
            }
        }
    }
}
