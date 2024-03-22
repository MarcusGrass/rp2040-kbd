use elite_pi::pac::UART0;
use embedded_hal::serial::{Read, Write};
use rp2040_hal::gpio::bank0::{Gpio0, Gpio1};
use rp2040_hal::gpio::{FunctionUart, Pin, PullDown};
use rp2040_hal::uart::{Enabled, UartPeripheral};

#[derive(Debug, Copy, Clone)]
pub enum SplitSerialMessage {
    Ping,
    Pong
}


pub struct SplitSerial {
    current: [u8; 16],
    current_offset: u8,
    driver: UartPeripheral<Enabled, UART0, (Pin<Gpio0, FunctionUart, PullDown>, Pin<Gpio1, FunctionUart, PullDown>)>,
}

impl SplitSerial {
    pub fn new(driver: UartPeripheral<Enabled, UART0, (Pin<Gpio0, FunctionUart, PullDown>, Pin<Gpio1, FunctionUart, PullDown>)>) -> Self {
        Self { current: [0u8; 16], current_offset: 0, driver }
    }

    pub fn send_msg(&mut self, msg: SplitSerialMessage) -> bool {
        let res = match msg {
            SplitSerialMessage::Ping => {
                self.driver.write(1).is_ok()
            }
            SplitSerialMessage::Pong => {
                self.driver.write(2).is_ok()
            }
        };
        if !res {
            return res;
        }
        self.driver.flush().is_ok()
    }

    pub fn recv(&mut self) -> Option<SplitSerialMessage> {
        loop {
            match self.driver.read() {
                Ok(byte) => {
                    self.current[self.current_offset as usize] = byte;
                    match self.current[0] {
                        1 => {
                            self.current_offset = 0;
                            self.current[0] = 0;
                            return Some(SplitSerialMessage::Ping);
                        }
                        2 => {
                            self.current_offset = 0;
                            self.current[0] = 0;
                            return Some(SplitSerialMessage::Pong);
                        }
                        _ => {
                            return None;
                        }
                    }
                }
                Err(e) => {
                    return None;
                }
            }
        }
    }

}

