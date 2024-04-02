use crate::keyboard::left::message_receiver::{DeserializedMessage, MessageReceiver};
use crate::keyboard::left::{KeyboardState, LeftButtons};
use crate::keyboard::oled::OledHandle;
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::split_serial::UartLeft;
use crate::keyboard::usb_serial::{UsbSerial, UsbSerialDevice};
use crate::keymap::{KeyboardReportState, KeymapLayer};
use crate::runtime::shared::usb::{acquire_usb, init_usb, push_hid_report, usb_hid_interrupt_poll};
use core::fmt::Write as _;
use embedded_hal::timer::CountDown;
use embedded_io::{Read, Write};
use heapless::String;
use liatris::pac::Interrupt::USBCTRL_IRQ;
use liatris::pac::{interrupt, Peripherals};
use nb::block;
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_hal::multicore::Multicore;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::Timer;
use usb_device::bus::{UsbBus, UsbBusAllocator};
use usbd_hid::descriptor::KeyboardReport;
use crate::keyboard::oled::left::LeftOledDrawer;
use crate::runtime::shared::cores_left::{KeycoreToAdminMessage, pop_message, push_loop_to_admin, push_touch_to_admin};
use crate::runtime::shared::loop_counter::LoopCounter;
use crate::runtime::shared::sleep::SleepCountdown;

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
    unsafe {
        init_usb(usb_bus);
    }
    let mut receiver = MessageReceiver::new(uart_driver);
    mc.cores()[1].spawn(unsafe { &mut CORE_1_STACK_AREA }, move || {
        run_core1(receiver, left_buttons, timer)
    });

    let mut oled_left = LeftOledDrawer::new(oled_handle);
    let mut last_chars = [0u8; 128];
    let mut output_all = false;
    let mut sleep = SleepCountdown::new();
    loop {
        let now = timer.get_counter();
        match pop_message() {
            Some(KeycoreToAdminMessage::Touch) => {
                sleep.touch(now);
                oled_left.show();
            }
            Some(KeycoreToAdminMessage::Loop(lc)) => {
                let loop_millis = lc.count as u64 / lc.duration.to_millis();
                if sleep.is_awake() {
                    if let Some((header, body)) = lc.as_display() {
                        oled_left.update_scan_loop(header, body);
                    }
                }
            }
            _ => {}
        }
        if sleep.should_sleep(now) {
            oled_left.hide();
        }
        oled_left.render();
        #[cfg(feature = "serial")]
        handle_usb(&mut power_led_pin, &mut last_chars, &mut output_all);
    }
}
#[cfg(feature = "serial")]
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

pub fn run_core1(mut receiver: MessageReceiver, mut left_buttons: LeftButtons, timer: Timer) -> ! {
    const DEFAULT_KBD: KeyboardReport = KeyboardReport {
        modifier: 0,
        reserved: 0,
        leds: 0,
        keycodes: [0u8; 6],
    };

    #[cfg(feature = "hiddev")]
    unsafe {
        liatris::hal::pac::NVIC::unmask(USBCTRL_IRQ);
    }
    let mut kbd = KeyboardState::empty();
    let mut kbd = crate::keymap::KeyboardState::new();
    let mut report_state = KeyboardReportState::new();
    let mut loop_count: LoopCounter<100_000> = LoopCounter::new(timer.get_counter());
    loop {
        let mut any_change = false;
        #[cfg(feature = "hiddev")]
        {
            if let Some(update) = receiver.try_read() {
                kbd.update_right(update, &mut report_state);
                any_change = true;
            }
            if kbd.scan_left(&mut left_buttons, &mut report_state) {
                any_change = true;
            }
            if any_change {
                push_hid_report(report_state.report());
                push_touch_to_admin();
            }
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
        if loop_count.increment() {
            let now = timer.get_counter();
            let lc = loop_count.value(now);
            if push_loop_to_admin(lc) {
                loop_count.reset(now);
            }
        }

    }
}

/// Safety: Called from the same core that publishes
#[interrupt]
#[allow(non_snake_case)]
#[cfg(feature = "hiddev")]
unsafe fn USBCTRL_IRQ() {
    usb_hid_interrupt_poll()
}
