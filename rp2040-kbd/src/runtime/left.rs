use crate::keyboard::left::message_receiver::MessageReceiver;
use crate::keyboard::left::LeftButtons;
use crate::keyboard::oled::left::{layer_to_string, LeftOledDrawer};
use crate::keyboard::oled::OledHandle;
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::split_serial::UartLeft;
use crate::keymap::{KeyboardReportState, KeymapLayer};
use crate::runtime::shared::cores_left::{
    new_shared_queue, pop_message, push_loop_to_admin, push_rx_change, push_touch_left_to_admin,
    push_touch_right_to_admin, Consumer, KeycoreToAdminMessage, Producer,
};
use crate::runtime::shared::loop_counter::LoopCounter;
use crate::runtime::shared::press_latency_counter::PressLatencyCounter;
use crate::runtime::shared::sleep::SleepCountdown;
#[cfg(feature = "serial")]
use crate::runtime::shared::usb::init_usb;
#[cfg(feature = "serial")]
use core::fmt::Write;
#[cfg(feature = "hiddev")]
use liatris::pac::interrupt;
use rp2040_hal::clocks::SystemClock;
use rp2040_hal::multicore::{Multicore, Stack};
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::{Clock, Timer};
use usb_device::bus::UsbBusAllocator;

static CORE_1_STACK: Stack<{ 1024 * 8 }> = Stack::new();

#[inline(never)]
pub fn run_left<'a>(
    mc: &'a mut Multicore<'a>,
    usb_bus: UsbBusAllocator<rp2040_hal::usb::UsbBus>,
    mut oled_handle: OledHandle,
    uart_driver: UartLeft,
    left_buttons: LeftButtons,
    power_led_pin: PowerLed,
    timer: Timer,
    system_clock: &SystemClock,
) -> ! {
    #[cfg(feature = "serial")]
    unsafe {
        init_usb(usb_bus);
    }
    let receiver = MessageReceiver::new(uart_driver);
    let (producer, consumer) = new_shared_queue();
    if let Err(_e) = mc.cores()[1].spawn(CORE_1_STACK.take().unwrap(), move || {
        run_key_processsing_core(
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
    run_admin_core(oled_handle, consumer, timer, power_led_pin, system_clock)
}

#[expect(clippy::needless_pass_by_value)]
pub fn run_admin_core(
    oled_handle: OledHandle,
    consumer: Consumer,
    timer: Timer,
    mut power_led_pin: PowerLed,
    sys_clock: &SystemClock,
) -> ! {
    let mut oled_left = LeftOledDrawer::new(oled_handle);
    #[cfg(feature = "serial")]
    let mut last_chars = [0u8; 128];
    #[cfg(feature = "serial")]
    let mut output_all = false;
    #[cfg(feature = "serial")]
    let mut has_dumped = false;
    let mut sleep = SleepCountdown::new();
    let mut rx: u16 = 0;
    let mut left_counter: PressLatencyCounter = PressLatencyCounter::new();
    let mut right_counter: PressLatencyCounter = PressLatencyCounter::new();
    let mut last_avail = 0;
    oled_left.update_layer(layer_to_string(KeymapLayer::DvorakSe));
    oled_left.set_clock(sys_clock.freq());
    loop {
        let avail = consumer.available();
        let now = timer.get_counter();
        match pop_message(&consumer) {
            Some(KeycoreToAdminMessage::TouchLeft(micros)) => {
                oled_left.update_left_counter(left_counter.increment_get_avg(micros));
                sleep.touch(now);
                power_led_pin.turn_on();
                oled_left.show();
            }
            Some(KeycoreToAdminMessage::TouchRight(micros)) => {
                oled_left.update_right_counter(right_counter.increment_get_avg(micros));
                sleep.touch(now);
                power_led_pin.turn_on();
                oled_left.show();
            }
            Some(KeycoreToAdminMessage::Loop(lc)) => {
                if sleep.is_awake() {
                    oled_left.update_scan_loop(lc.as_micros_fraction());
                }
            }
            Some(KeycoreToAdminMessage::LayerChange(default)) => {
                let dfl_out = layer_to_string(default);
                oled_left.update_layer(dfl_out);
            }
            Some(KeycoreToAdminMessage::Rx(incr)) => {
                rx += incr;
                if rx > 9999 {
                    rx = incr;
                }
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
        if avail != last_avail {
            oled_left.update_queue(avail);
            last_avail = avail;
        }
        if sleep.should_sleep(now) {
            oled_left.hide();
            power_led_pin.turn_off();
            sleep.set_sleeping();
        }
        oled_left.render();
        #[cfg(feature = "serial")]
        {
            handle_usb(
                &mut power_led_pin,
                &mut last_chars,
                &mut output_all,
                sys_clock,
            );
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
    clocks: &SystemClock,
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
                    } else if last_chars.ends_with(b"CLOCK") {
                        let _ =
                            serial.write_fmt(format_args!("SYS={}\r\n", clocks.freq().to_MHz(),));
                    }
                }
            }
        }
    }
    Some(())
}

#[expect(clippy::needless_pass_by_value)]
pub fn run_key_processsing_core(
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
        let loop_timer = timer.get_counter();
        let mut changed_left = false;
        let mut changed_right = false;
        if let Some(update) = receiver.try_read() {
            // Right side sent an update
            rx += 1;
            // Update report state
            kbd.update_right(update, &mut report_state, &producer);
            changed_right = true;
        }
        // Check left side gpio and update report state
        if kbd.scan_left(&mut left_buttons, &mut report_state, timer, &producer) {
            changed_left = true;
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
        if let Some(dur) = timer.get_counter().checked_duration_since(loop_timer) {
            if changed_left {
                push_touch_left_to_admin(&producer, dur);
            }
            if changed_right {
                push_touch_right_to_admin(&producer, dur);
            }
        }
    }
}

/// Todo: Change to 'expect' after [this PR](https://github.com/rust-embedded/cortex-m/pull/557)
/// Safety: Called from the same core that publishes
#[interrupt]
#[allow(clippy::allow_attributes, non_snake_case)]
#[cfg(feature = "hiddev")]
unsafe fn USBCTRL_IRQ() {
    crate::runtime::shared::usb::hiddev_interrupt_poll();
}
