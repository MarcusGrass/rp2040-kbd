use elite_pi::pac::{PIO0, UART0};
use embedded_hal::digital::v2::{InputPin, OutputPin, PinState};
use embedded_hal::serial::{Read};
use embedded_hal::timer::CountDown;
use embedded_io::Write;
use nb::block;
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_hal::gpio::bank0::{Gpio0, Gpio1, Gpio2};
use rp2040_hal::gpio::{FunctionSio, FunctionUart, Pin, PullBusKeep, PullDown, PullUp, SioInput, SioOutput};
use rp2040_hal::pio::Running;
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

const HOLD_TIME_MICROS: u64 = 20;

impl SplitSerial {
    pub fn new(mut pin: Pin<Gpio1, FunctionSio<SioOutput>, PullBusKeep>, timer: Timer) -> Self {
        let _ = pin.set_high();
        Self { current: [0u8; 16], current_offset: 0, pin, timer }
    }

    pub fn read_pin(&mut self) -> bool {
        matches!(self.pin.is_high(), Ok(true))
    }

    pub fn set_pin(&mut self) -> bool {
        self.pin.set_high().is_ok()

    }

    pub fn unset_pin(&mut self) -> bool {
        self.pin.set_low().is_ok()
    }

}


pub struct UartLeft {
    pub(crate) inner: pio_uart::PioUart<Gpio0, Gpio1, PIO0, Running>
}

impl UartLeft {
    pub fn new(inner: pio_uart::PioUart<Gpio0, Gpio1, PIO0, Running>) -> Self {
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
