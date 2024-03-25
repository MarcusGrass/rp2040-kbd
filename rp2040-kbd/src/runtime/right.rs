mod shared;

use core::fmt::Write;
use embedded_hal::timer::CountDown;
use embedded_io::Read;
use heapless::String;
use nb::block;
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_hal::multicore::Multicore;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::Timer;
use usb_device::bus::UsbBus;
use crate::keyboard::oled::{OledHandle};
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::right::message_serializer::MessageSerializer;
use crate::keyboard::right::RightButtons;
use crate::keyboard::split_serial::{UartRight};
use crate::keyboard::usb_serial::{UsbSerial, UsbSerialDevice};
use crate::runtime::right::shared::{acquire_matrix_scan, try_acquire_matrix_scan};

static mut CORE_1_STACK_AREA: [usize; 1024] = [0; 1024];

#[inline(never)]
pub fn run_right<'a>(mc: &'a mut Multicore<'a>, mut usb_serial: UsbSerial, mut usb_dev: UsbSerialDevice, mut oled_handle: OledHandle, uart_driver: UartRight, mut right_buttons: RightButtons, mut power_led_pin: PowerLed, timer: Timer) -> !{
    const PING: &[u8] = b"ping";

    let cores = mc.cores();
    let c1 = &mut cores[1];
    let mut serializer = MessageSerializer::new(uart_driver);
    c1.spawn(unsafe {&mut CORE_1_STACK_AREA}, move || run_core1(serializer, right_buttons)).unwrap();
    let mut last_chars = [0u8; 128];
    let mut output_all = false;
    let mut has_dumped = false;
    let mut prev = timer.get_counter();
    let mut matrix_sends = 0u16;
    let mut pump_failures = 0u16;
    let mut total_read = 0u16;
    let mut total_written = 0u16;
    let mut errs = 0u16;
    let mut buf = [0u8; 64];
    let mut offset = 0;
    let mut loop_counter = timer.get_counter();
    let mut avg_loop = 0f32;
    let mut num_loops: f32 = 0.0;
    let mut scan_counter = timer.get_counter();
    let mut avg_scan = 0f32;
    let mut num_scans: f32 = 0.0;
    oled_handle.clear();
    loop {

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
        }
        let now = timer.get_counter();
        if let Some(dur) = now.checked_duration_since(prev) {
            if dur.to_millis() > 200 {
                prev = now;
                let mut s: String<5> = String::new();

                if s.write_fmt(format_args!("{matrix_sends}")).is_ok() {
                    oled_handle.clear_line(0);
                    let _ = oled_handle.write(0, s.as_str());
                }
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{pump_failures}")).is_ok() {
                    oled_handle.clear_line(18);
                    let _ = oled_handle.write(18, s.as_str());
                }
                let mut time_text: String<5> = String::new();
                if time_text.write_fmt(format_args!("{avg_loop:.1}")).is_ok() {
                    oled_handle.clear_line(36);
                    let _ = oled_handle.write(36, time_text.as_str());
                }
                let mut time_text: String<5> = String::new();
                if time_text.write_fmt(format_args!("{avg_scan:.1}")).is_ok() {
                    oled_handle.clear_line(54);
                    let _ = oled_handle.write(54, time_text.as_str());
                }
            }
        }
        if num_loops >= f32::MAX - 1.0 {
            num_loops = 0.0;
            avg_loop = 0.0;
        }
        num_loops += 1.0;
        if let Some(loop_cnt) = now.checked_duration_since(loop_counter) {
            let time = loop_cnt.to_micros() as f32;
            avg_loop = avg_loop * ((num_loops - 1.0) / num_loops) + time / num_loops;
            loop_counter = now;
        }
        let Some(ms) = try_acquire_matrix_scan() else {
            continue;
        };
        let count = core::mem::replace(&mut ms.scan.num_scans, 0);
        drop(ms);
        if count >= f32::MAX as usize - 2 {
            num_scans = 0.0;
            avg_scan = 0.0;
        } else {
            let count_f = count as f32;
            if count_f >= f32::MAX - num_scans - 1.0 {
                num_scans = 0.0;
                avg_scan = 0.0;
            }
        }
        let count_f = count as f32;
        num_scans += count_f;
        if let Some(scan_timer) = now.checked_duration_since(scan_counter) {
            let scan_time = scan_timer.to_millis();
            if scan_time >= f32::MAX as u64 - 1 {
                scan_counter = timer.get_counter();
                continue;
            } else if scan_time == 0 {
                continue;
            }
            let scan_timef = scan_time as f32;
            avg_scan = scan_timef / num_scans;
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

fn run_core1(mut serializer: MessageSerializer, mut right_buttons: RightButtons) -> ! {
    loop {
        let mut pumped = false;
        right_buttons.scan_matrix();
        if serializer.serialize_matrix_state(&right_buttons.matrix) {
        } else if serializer.pump() {
            pumped = true;
            // Successfully cleared old data
            if serializer.serialize_matrix_state(&right_buttons.matrix) {
            } else {
                // Give up on this one
                serializer.clear();
            }
        }
        if !pumped {
            pumped = serializer.pump();
        }
        acquire_matrix_scan().scan.num_scans += 1;
    }
}