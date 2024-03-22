use elite_pi::pac::UART0;
use embedded_hal::serial::{Read, Write};
use rp2040_hal::gpio::bank0::{Gpio0, Gpio1};
use rp2040_hal::gpio::{FunctionUart, Pin, PullDown};
use rp2040_hal::uart::{Enabled, UartPeripheral};

pub enum UartSerialRequest {
    Ping,
}

pub enum UartSerialResponse {
    Pong,
}

pub struct UartSerial {
    current: [u8; 16],
    current_offset: u8,
    driver: UartPeripheral<Enabled, UART0, (Pin<Gpio0, FunctionUart, PullDown>, Pin<Gpio1, FunctionUart, PullDown>)>,
}

impl UartSerial {
    pub fn new(driver: UartPeripheral<Enabled, UART0, (Pin<Gpio0, FunctionUart, PullDown>, Pin<Gpio1, FunctionUart, PullDown>)>) -> Self {
        Self { current: [0u8; 16], current_offset: 0, driver }
    }

    pub fn send_msg(&mut self, msg: UartSerialRequest) {
        match msg {
            UartSerialRequest::Ping => {
                self.driver.write(1).unwrap()
            }
        }
    }

    pub fn recv(&mut self) -> Option<UartSerialResponse> {
        loop {
            match self.driver.read() {
                Ok(byte) => {
                    self.current[self.current_offset as usize] = byte;
                    match self.current[0] {
                        1 => {
                            self.current_offset = 0;
                            self.current[0] = 0;
                            return Some(UartSerialResponse::Pong);
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

