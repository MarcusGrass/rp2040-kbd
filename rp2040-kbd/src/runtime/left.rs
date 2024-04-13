use crate::keyboard::left::message_receiver::MessageReceiver;
use crate::keyboard::left::LeftButtons;
use crate::keyboard::oled::left::LeftOledDrawer;
use crate::keyboard::oled::OledHandle;
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::split_serial::UartLeft;
use crate::keymap::{KeyboardReportState, KeymapLayer};
use crate::runtime::shared::cores_left::{
    pop_message, push_layer_change, push_loop_to_admin, push_rx_change, push_touch_to_admin,
    KeycoreToAdminMessage, Producer,
};
use crate::runtime::shared::loop_counter::LoopCounter;
use crate::runtime::shared::sleep::SleepCountdown;
#[cfg(feature = "serial")]
use crate::runtime::shared::usb::init_usb;
#[cfg(feature = "serial")]
use core::fmt::Write;
use core::sync::atomic::AtomicUsize;
use heapless::String;
#[cfg(feature = "hiddev")]
use liatris::pac::interrupt;
use rp2040_hal::multicore::Multicore;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::Timer;
use rp2040_kbd_lib::queue::new_atomic_producer_consumer;
use usb_device::bus::UsbBusAllocator;

static mut CORE_1_STACK_AREA: [usize; 1024 * 8] = [0; 1024 * 8];

static mut ATOMIC_QUEUE_MEM_AREA: [KeycoreToAdminMessage; 32] = [KeycoreToAdminMessage::Touch; 32];
static mut ATOMIC_QUEUE_HEAD: AtomicUsize = AtomicUsize::new(0);
static mut ATOMIC_QUEUE_TAIL: AtomicUsize = AtomicUsize::new(0);
#[inline(never)]
#[allow(clippy::too_many_lines)]
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
    let (producer, consumer) = unsafe {
        new_atomic_producer_consumer(
            &mut ATOMIC_QUEUE_MEM_AREA,
            &mut ATOMIC_QUEUE_HEAD,
            &mut ATOMIC_QUEUE_TAIL,
        )
    };
    #[allow(static_mut_refs)]
    if let Err(_e) = mc.cores()[1].spawn(unsafe { &mut CORE_1_STACK_AREA }, move || {
        run_core1(
            receiver,
            left_buttons,
            timer,
            producer,
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
        panic!();
    }

    let mut oled_left = LeftOledDrawer::new(oled_handle);
    #[cfg(feature = "serial")]
    let mut last_chars = [0u8; 128];
    #[cfg(feature = "serial")]
    let mut output_all = false;
    #[cfg(feature = "serial")]
    let mut has_dumped = false;
    let mut sleep = SleepCountdown::new();
    let mut rx: u16 = 0;
    loop {
        let now = timer.get_counter();
        match pop_message(&consumer) {
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
            Some(KeycoreToAdminMessage::Rx(incr)) => {
                rx = rx.wrapping_add(incr);
                sleep.touch(now);
                oled_left.update_rx(rx);
            }
            Some(KeycoreToAdminMessage::Reboot) => {
                oled_left.render_boot_msg();
                reset_to_usb_boot(0, 0);
                panic!("HALT POST RESET");
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
            if output_all && !has_dumped {
                let _ =
                    crate::runtime::shared::usb::acquire_usb().write_str("Left side running\r\n");
                has_dumped = true;
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
                        let _ = serial.write_str("BOOT\r\n");
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
#[allow(clippy::needless_pass_by_value)]
pub fn run_core1(
    mut receiver: MessageReceiver,
    mut left_buttons: LeftButtons,
    timer: Timer,
    producer: Producer,
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
    let mut loop_count: LoopCounter<10_000> = LoopCounter::new(timer.get_counter());
    #[cfg(feature = "hiddev")]
    unsafe {
        liatris::hal::pac::NVIC::unmask(liatris::pac::Interrupt::USBCTRL_IRQ);
    }
    let mut rx = 0;
    loop {
        let mut any_change = false;
        if let Some(update) = receiver.try_read() {
            // Right side sent an update
            rx += 1;
            // Update report state
            kbd.update_right(update, &mut report_state, &producer);
            any_change = true;
        }
        // Check left side gpio and update report state
        if kbd.scan_left(&mut left_buttons, &mut report_state, timer, &producer) {
            any_change = true;
        }
        if any_change {
            push_touch_to_admin(&producer);
        }
        #[cfg(feature = "hiddev")]
        {
            let mut pop = false;
            if let Some(next_update) = report_state.report() {
                // Published the next update on queue if present
                unsafe {
                    pop = crate::runtime::shared::usb::try_push_report(next_update);
                }
            }
            if pop {
                // Remove the sent report (it's down here because of the borrow checker)
                report_state.accept();
            }
        }

        if let Some(change) = report_state.layer_update() {
            push_layer_change(&producer, change);
        }

        if rx > 0 && push_rx_change(&producer, rx) {
            rx = 0;
        }
        if loop_count.increment() {
            let now = timer.get_counter();
            let lc = loop_count.value(now);
            if push_loop_to_admin(&producer, lc) {
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
    crate::runtime::shared::usb::hiddev_interrupt_poll();
}
