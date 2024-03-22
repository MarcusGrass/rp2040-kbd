use core::fmt::Write;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::Timer;
use usb_device::bus::UsbBus;
use crate::keyboard::left::LeftButtons;
use crate::keyboard::oled::{OledHandle};
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::uart_serial::{SplitSerial, SplitSerialMessage};
use crate::keyboard::usb_serial::{UsbSerial, UsbSerialDevice};

#[inline(never)]
pub fn run_left(mut usb_serial: UsbSerial, mut usb_dev: UsbSerialDevice, _oled_handle: OledHandle, mut uart_driver: SplitSerial, mut left_buttons: LeftButtons, mut power_led_pin: PowerLed, timer: Timer) -> !{
    let mut last_chars = [0u8; 128];
    let mut output_all = false;
    let mut has_dumped = false;
    let mut prev = timer.get_counter();
    let mut has_sent_ping = false;
    loop {
        let now = timer.get_counter();
        if let Some(dur) = now.checked_duration_since(prev) {
            if dur.to_millis() > 200 && output_all {
                if !has_sent_ping {
                    uart_driver.send_msg(SplitSerialMessage::Ping);
                    let _ = usb_serial.write_str("Sent ping\r\n");
                    has_sent_ping = true;
                } else {
                    let recv = uart_driver.recv();
                    let _ = usb_serial.write_fmt(format_args!("Got msg: {recv:?}\r\n"));
                    has_sent_ping = matches!(recv, Some(SplitSerialMessage::Pong));
                }
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
            for change in left_buttons.scan_matrix() {
                let _ = usb_serial.write_fmt(format_args!("{change:?}\r\n"));
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