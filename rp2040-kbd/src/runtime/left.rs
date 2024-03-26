mod usb;
mod shared;

use core::fmt::Write as _;
use embedded_hal::timer::CountDown;
use embedded_io::{Read, Write};
use heapless::String;
use nb::block;
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::Timer;
use usb_device::bus::{UsbBus, UsbBusAllocator};
use crate::keyboard::left::{KeyboardState, LeftButtons};
use crate::keyboard::left::message_receiver::{DeserializedMessage, MessageReceiver};
use crate::keyboard::oled::{OledHandle};
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::split_serial::{UartLeft};
use crate::keyboard::usb_serial::{UsbSerial, UsbSerialDevice};

#[inline(never)]
pub fn run_left(mut usb_bus: UsbBusAllocator<rp2040_hal::usb::UsbBus>, mut oled_handle: OledHandle, mut uart_driver: UartLeft, mut left_buttons: LeftButtons, mut power_led_pin: PowerLed, timer: Timer) -> !{
    const PONG: &[u8] = b"pong";
    let mut usb_serial = UsbSerial::new(&usb_bus);
    let mut usb_dev = UsbSerialDevice::new(&usb_bus);
    let mut receiver = MessageReceiver::new(uart_driver);
    let mut last_chars = [0u8; 128];
    let mut output_all = false;
    let mut has_dumped = false;
    let mut wants_read = false;
    let mut next_u8 = 0u8;
    let mut prev = timer.get_counter();
    let mut flips = 0u16;
    let mut buf = [0u8; 64];
    let mut offset = 0;
    let mut read = 0u16;
    let mut written = 0u16;
    let mut empty_reads = 0u16;
    let mut err_reads = 0u16;
    let mut kbd: KeyboardState<0> = KeyboardState::empty();
    loop {
        let now = timer.get_counter();
        if let Some(dur) = now.checked_duration_since(prev) {
            if dur.to_millis() > 1000 && output_all {
                oled_handle.clear();
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{flips}")).is_ok() {
                    oled_handle.write(0, s.as_str());
                }
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{read}")).is_ok() {
                    oled_handle.write(18, s.as_str());
                }
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{empty_reads}")).is_ok() {
                    oled_handle.write(36, s.as_str());
                }
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{err_reads}")).is_ok() {
                    oled_handle.write(54, s.as_str());
                }
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{}", offset as u16)).is_ok() {
                    oled_handle.write(74, s.as_str());
                }
                wants_read = false;
                prev = now;

            }
        }
        handle_usb(
            &mut usb_dev,
            &mut usb_serial,
            &mut power_led_pin,
            &mut last_chars,
            &mut output_all,
        );
        if output_all {
            if let Some(input) = receiver.try_read() {
                match input {
                    DeserializedMessage::Matrix(m) => {
                        kbd.update_right(m, &mut usb_serial);
                    }
                    DeserializedMessage::Encoder(_) => {}
                }
                //let _ = usb_serial.write_fmt(format_args!("Got message: {input:?}\r\n"));
            }

            for press in left_buttons.scan_matrix() {
                let _ = usb_serial.write_fmt(format_args!("Btn: {press:?}\r\n"));
            }
        }

    }
}
fn handle_usb(
    usb_dev: &mut UsbSerialDevice,
    serial: &mut UsbSerial,
    power_led: &mut PowerLed,
    last_chars: &mut [u8],
    output_all: &mut bool,
) {
    if usb_dev.inner.poll(&mut [&mut serial.inner]) {
        let last_chars_len = last_chars.len();
        let mut buf = [0u8; 64];
        match serial.inner.read(&mut buf) {
            Err(_e) => {
                // Do nothing
            }
            Ok(0) => {
                // Do nothing
            }
            Ok(count) => {
                for byte in &buf[..count] {
                    last_chars.copy_within(1..last_chars_len, 0);
                    last_chars[last_chars.len() - 1] = *byte;
                    if last_chars.ends_with(b"boot") {
                        reset_to_usb_boot(0, 0);
                    } else if last_chars.ends_with(b"output") {
                        *output_all = true;
                    } else if last_chars.ends_with(b"led") {
                        if power_led.is_on() {
                            power_led.turn_off();
                        } else {
                            power_led.turn_on();
                        }
                    }
                }
            }
        }
    }
}
