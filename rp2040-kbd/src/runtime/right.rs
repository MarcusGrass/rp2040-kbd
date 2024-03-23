use core::fmt::Write;
use embedded_hal::timer::CountDown;
use embedded_io::Read;
use heapless::String;
use nb::block;
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::Timer;
use usb_device::bus::UsbBus;
use crate::keyboard::oled::{OledHandle};
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::right::RightButtons;
use crate::keyboard::split_serial::{serial_delay, SplitSerial, SplitSerialMessage, UartRight};
use crate::keyboard::usb_serial::{UsbSerial, UsbSerialDevice};

#[inline(never)]
pub fn run_right(mut usb_serial: UsbSerial, mut usb_dev: UsbSerialDevice, mut oled_handle: OledHandle, mut uart_driver: UartRight, mut right_buttons: RightButtons, mut power_led_pin: PowerLed, timer: Timer) -> !{
    let mut last_chars = [0u8; 128];
    let mut output_all = false;
    let mut has_dumped = false;
    let mut prev = timer.get_counter();
    let mut flips = 0u16;
    let mut total_read = 0u16;
    let mut total_written = 0u16;
    let mut errs = 0u16;
    let mut buf = [0u8; 64];
    let mut offset = 0;
    loop {
        let now = timer.get_counter();
        if let Some(dur) = now.checked_duration_since(prev) {
            if dur.to_millis() > 200 {
                oled_handle.clear();
                prev = now;
                let mut s: String<5> = String::new();

                if s.write_fmt(format_args!("{flips}")).is_ok() {
                    let _ = oled_handle.write(0, s.as_str());
                }
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{total_read}")).is_ok() {
                    let _ = oled_handle.write(18, s.as_str());
                }
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{total_written}")).is_ok() {
                    let _ = oled_handle.write(36, s.as_str());
                }
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{errs}")).is_ok() {
                    let _ = oled_handle.write(45, s.as_str());
                }
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
            if !has_dumped {
                let _ = usb_serial.write_str("Right side running\r\n");
                has_dumped = true;
            }
            for change in right_buttons.scan_matrix() {
                let _ = usb_serial.write_fmt(format_args!("{change:?}\r\n"));
            }

        }
        if let Ok(r) = uart_driver.inner.read(&mut buf) {
            total_read += r as u16;
            if r == 0 {
                continue;
            }
            offset += r;
            if offset >= buf.len() {
                offset = 0;
            } else {
                let expect = b"ping";
                if &buf[..expect.len()] == expect {
                    flips = flips.wrapping_add(1);
                }
                offset = 0;
                if uart_driver.write_all(b"pong") {
                    total_written = total_written.wrapping_add(b"pong".len() as u16);
                } else {
                    errs = errs.wrapping_add(1);
                }
            }
        } else {
            errs = errs.wrapping_add(1);
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