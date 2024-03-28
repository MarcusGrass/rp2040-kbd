mod shared;
mod usb;

use crate::keyboard::left::message_receiver::{DeserializedMessage, MessageReceiver};
use crate::keyboard::left::{KeyboardState, LeftButtons};
use crate::keyboard::oled::OledHandle;
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::split_serial::UartLeft;
use crate::keyboard::usb_serial::{UsbSerial, UsbSerialDevice};
use crate::keymap::Layers;
use crate::runtime::shared::usb::{acquire_usb, init_usb, push_hid_report, usb_hid_interrupt_poll};
use core::fmt::Write as _;
use embedded_hal::timer::CountDown;
use embedded_io::{Read, Write};
use heapless::String;
use liatris::pac::{interrupt, Peripherals};
use liatris::pac::Interrupt::USBCTRL_IRQ;
use nb::block;
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_hal::multicore::Multicore;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::Timer;
use usb_device::bus::{UsbBus, UsbBusAllocator};
use usbd_hid::descriptor::KeyboardReport;

static mut CORE_1_STACK_AREA: [usize; 1024] = [0; 1024];
#[inline(never)]
pub fn run_left<'a>(
    mc: &'a mut Multicore<'a>,
    mut usb_bus: UsbBusAllocator<rp2040_hal::usb::UsbBus>,
    mut oled_handle: OledHandle,
    mut uart_driver: UartLeft,
    mut left_buttons: LeftButtons,
    mut power_led_pin: PowerLed,
    timer: Timer,
) -> ! {
    const PONG: &[u8] = b"pong";
    unsafe {
        init_usb(usb_bus);
    }
    let mut receiver = MessageReceiver::new(uart_driver);
    mc.cores()[1].spawn(unsafe { &mut CORE_1_STACK_AREA }, || {
        run_core1(receiver, left_buttons)
    });
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
        /*
        if let Some(dur) = now.checked_duration_since(prev) {
            if dur.to_millis() > 1000 && output_all {
                oled_handle.clear();
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{}", receiver.good_matrix)).is_ok() {
                    oled_handle.write(0, s.as_str());
                }
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{}", receiver.successful_reads)).is_ok() {
                    oled_handle.write(18, s.as_str());
                }
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{}", receiver.total_read)).is_ok() {
                    oled_handle.write(36, s.as_str());
                }
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{}", receiver.bad_matrix)).is_ok() {
                    oled_handle.write(54, s.as_str());
                }
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{}", receiver.unk_msg)).is_ok() {
                    oled_handle.write(74, s.as_str());
                }
                let mut s: String<5> = String::new();
                if s.write_fmt(format_args!("{}", receiver.unk_rollback)).is_ok() {
                    oled_handle.write(92, s.as_str());
                }
                wants_read = false;
                prev = now;

            }
        }

         */
        handle_usb(&mut power_led_pin, &mut last_chars, &mut output_all);
    }
}
fn handle_usb(
    power_led: &mut PowerLed,
    last_chars: &mut [u8],
    output_all: &mut bool,
) -> Option<()> {
    let mut usb = acquire_usb();
    let mut serial = usb.serial?;
    let mut dev = usb.dev?;
    if dev.inner.poll(&mut [&mut serial.inner]) {
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
                        *usb.output = true;
                        let _ = serial.write_str("OUTPUT ON\r\n");
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
    Some(())
}

pub fn run_core1(mut receiver: MessageReceiver, mut left_buttons: LeftButtons) -> ! {
    const DEFAULT_KBD: KeyboardReport = KeyboardReport {
        modifier: 0,
        reserved: 0,
        leds: 0,
        keycodes: [0u8; 6],
    };

    // Handle interrupt on this same core
    #[cfg(feature = "hiddev")]
    unsafe {
        liatris::hal::pac::NVIC::unmask(USBCTRL_IRQ);
    }
    let mut kbd = KeyboardState::empty();
    loop {
        let mut any_change = false;
        if let Some(update) = receiver.try_read() {
            any_change = kbd.update_right(update);
        }

        if left_buttons.scan_matrix() {
            any_change = true;
        }
        let next_layer = Layers::DvorakAnsi.report(&left_buttons.matrix, &kbd.right);
        #[cfg(feature = "hiddev")]
        {
            push_hid_report(next_layer.report);
        }
        #[cfg(feature = "serial")]
        {
            /*
            if next_layer.report.keycodes != DEFAULT_KBD.keycodes
                || next_layer.report.modifier != DEFAULT_KBD.modifier
            {
                let _ = acquire_usb().write_fmt(format_args!("Report: {:?}\r\n", next_layer));
            }

             */
        }

    }
}

#[interrupt]
#[allow(non_snake_case)]
#[cfg(feature = "hiddev")]
unsafe fn USBCTRL_IRQ() {
    usb_hid_interrupt_poll()
}
