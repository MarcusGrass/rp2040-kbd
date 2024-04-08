use crate::keyboard::left::message_receiver::MessageReceiver;
use crate::keyboard::left::LeftButtons;
use crate::keyboard::oled::left::LeftOledDrawer;
use crate::keyboard::oled::OledHandle;
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::split_serial::UartLeft;
use crate::keymap::{KeyboardReportState, KeymapLayer};
use crate::runtime::shared::cores_left::{
    pop_message, push_layer_change, push_loop_to_admin, push_touch_to_admin, KeycoreToAdminMessage,
};
use crate::runtime::shared::loop_counter::LoopCounter;
use crate::runtime::shared::sleep::SleepCountdown;
#[cfg(feature = "serial")]
use crate::runtime::shared::usb::init_usb;
#[cfg(feature = "serial")]
use core::fmt::Write;
use heapless::String;
#[cfg(feature = "hiddev")]
use liatris::pac::interrupt;
use rp2040_hal::multicore::Multicore;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::Timer;
use usb_device::bus::UsbBusAllocator;

static mut CORE_1_STACK_AREA: [usize; 1024 * 8] = [0; 1024 * 8];
#[inline(never)]
pub fn run_left<'a>(
    mc: &'a mut Multicore<'a>,
    usb_bus: UsbBusAllocator<rp2040_hal::usb::UsbBus>,
    mut oled_handle: OledHandle,
    uart_driver: UartLeft,
    left_buttons: LeftButtons,
    #[allow(unused_variables, unused_mut)] mut power_led_pin: PowerLed,
    timer: Timer,
) -> ! {
    #[cfg(feature = "serial")]
    unsafe {
        init_usb(usb_bus);
    }
    let receiver = MessageReceiver::new(uart_driver);
    #[allow(static_mut_refs)]
    if let Err(_e) = mc.cores()[1].spawn(unsafe { &mut CORE_1_STACK_AREA }, move || {
        run_core1(
            receiver,
            left_buttons,
            timer,
            #[cfg(feature = "hiddev")]
            usb_bus,
        )
    }) {
        oled_handle.clear();
        oled_handle.write(0, "ERROR");
        oled_handle.write(9, "SPAWN");
        oled_handle.write(18, "CORE1");
        oled_handle.write(27, "FAIL");
        oled_handle.write(36, "BOOT");
        reset_to_usb_boot(0, 0);
    }

    let mut oled_left = LeftOledDrawer::new(oled_handle);
    #[cfg(feature = "serial")]
    let mut last_chars = [0u8; 128];
    #[cfg(feature = "serial")]
    let mut output_all = false;
    #[cfg(feature = "serial")]
    let mut has_dumped = false;
    let mut sleep = SleepCountdown::new();
    loop {
        let now = timer.get_counter();
        match pop_message() {
            Some(KeycoreToAdminMessage::Touch) => {
                sleep.touch(now);
                oled_left.show();
            }
            Some(KeycoreToAdminMessage::Loop(lc)) => {
                if sleep.is_awake() {
                    if let Some((header, body)) = lc.as_display() {
                        oled_left.update_scan_loop(header, body);
                    }
                }
            }
            Some(KeycoreToAdminMessage::LayerChange(km)) => {
                let mut s = String::new();
                match km {
                    KeymapLayer::DvorakSe => {
                        let _ = s.push_str("DV-SE");
                    }
                    KeymapLayer::DvorakAnsi => {
                        let _ = s.push_str("DV-AN");
                    }
                    KeymapLayer::QwertyAnsi => {
                        let _ = s.push_str("QW-AN");
                    }
                    KeymapLayer::QwertyGaming => {
                        let _ = s.push_str("QW-GM");
                    }
                    KeymapLayer::Lower => {
                        let _ = s.push_str("LO");
                    }
                    KeymapLayer::LowerAnsi => {
                        let _ = s.push_str("LO-AN");
                    }
                    KeymapLayer::Raise => {
                        let _ = s.push_str("RA");
                    }
                    KeymapLayer::Num => {
                        let _ = s.push_str("NUM");
                    }
                    KeymapLayer::Settings => {
                        let _ = s.push_str("SET");
                    }
                }
                oled_left.update_layer(s);
            }
            _ => {}
        }
        if sleep.should_sleep(now) {
            oled_left.hide();
            sleep.set_sleeping();
        }
        oled_left.render();
        #[cfg(feature = "serial")]
        {
            handle_usb(&mut power_led_pin, &mut last_chars, &mut output_all);
            if output_all {
                if !has_dumped {
                    let _ = crate::runtime::shared::usb::acquire_usb()
                        .write_str("Left side running\r\n");
                    has_dumped = true;
                }
            }
        }
    }
}
#[cfg(feature = "serial")]
fn handle_usb(
    power_led: &mut PowerLed,
    last_chars: &mut [u8],
    output_all: &mut bool,
) -> Option<()> {
    let usb = crate::runtime::shared::usb::acquire_usb();
    let serial = usb.serial?;
    let dev = usb.dev?;
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

pub fn run_core1(
    mut receiver: MessageReceiver,
    mut left_buttons: LeftButtons,
    timer: Timer,
    #[cfg(feature = "hiddev")] allocator: usb_device::bus::UsbBusAllocator<
        liatris::hal::usb::UsbBus,
    >,
) -> ! {
    #[cfg(feature = "hiddev")]
    unsafe {
        crate::runtime::shared::usb::init_usb_hiddev(allocator);
    }
    let mut kbd = crate::keymap::KeyboardState::new();
    let mut report_state = KeyboardReportState::new();
    let mut loop_count: LoopCounter<100_000> = LoopCounter::new(timer.get_counter());
    #[cfg(feature = "hiddev")]
    unsafe {
        liatris::hal::pac::NVIC::unmask(liatris::pac::Interrupt::USBCTRL_IRQ);
    }
    loop {
        let mut any_change = false;
        if let Some(update) = receiver.try_read() {
            kbd.update_right(update, &mut report_state);
            any_change = true;
        }
        if kbd.scan_left(&mut left_buttons, &mut report_state, timer) {
            any_change = true;
        }
        if any_change {
            push_touch_to_admin();
        }
        #[cfg(feature = "hiddev")]
        {
            let mut pop = false;
            if let Some(next_update) = report_state.report() {
                unsafe {
                    pop = crate::runtime::shared::usb::try_push_report(next_update);
                }
            }
            if pop {
                report_state.accept();
            }
        }

        if let Some(change) = report_state.layer_update() {
            push_layer_change(change);
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
    crate::runtime::shared::usb::hiddev_interrupt_poll()
}
