use core::fmt::Write as _;
use embedded_hal::timer::CountDown;
use embedded_io::{Read, Write};
use heapless::String;
use nb::block;
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::Timer;
use usb_device::bus::UsbBus;
use crate::keyboard::left::LeftButtons;
use crate::keyboard::oled::{OledHandle};
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::split_serial::{serial_delay, SplitSerial, SplitSerialMessage, UartLeft};
use crate::keyboard::usb_serial::{UsbSerial, UsbSerialDevice};

#[inline(never)]
pub fn run_left(mut usb_serial: UsbSerial, mut usb_dev: UsbSerialDevice, mut oled_handle: OledHandle, mut uart_driver: UartLeft, mut left_buttons: LeftButtons, mut power_led_pin: PowerLed, timer: Timer) -> !{
    const PONG: &[u8] = b"pong";
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
            if !has_dumped {
                let _ = usb_serial.write_str("Left side running\r\n");
                has_dumped = true;
            }
            /*
            for change in left_buttons.scan_matrix() {
                let _ = usb_serial.write_fmt(format_args!("{change:?}\r\n"));
            }

             */
            if wants_read {
                if let Ok(r) = uart_driver.inner.read(&mut buf[offset..]) {
                    read += r as u16;
                    if r == 0 {
                        empty_reads += 1;
                        continue;
                    }
                    let _ = usb_serial.write_fmt(format_args!("Read {r} bytes\r\n"));
                    offset += r;
                    if offset >= buf.len() {
                        // Safety reset
                        offset = 0;
                    }
                    let expect = PONG;
                    if &buf[..expect.len()] == PONG {
                        wants_read = false;
                        let _ = usb_serial.write_str("Got pong\r\n");
                    } else {
                        err_reads += 1;
                    }
                } else {
                    err_reads += 1;
                }
            } else {
                if uart_driver.write_all(b"ping") {
                    flips += 1;
                    wants_read = true;
                } else {
                    let _ = usb_serial.write_str("Failed write\r\n");
                }


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