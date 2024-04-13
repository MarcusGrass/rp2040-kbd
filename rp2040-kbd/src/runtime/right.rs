use crate::keyboard::oled::right::RightOledDrawer;
use crate::keyboard::oled::OledHandle;
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::right::message_serializer::MessageSerializer;
use crate::keyboard::right::RightButtons;
use crate::runtime::shared::cores_right::{
    new_shared_queue, pop_message, push_loop_to_admin, try_push_touch, KeycoreToAdminMessage,
    Producer,
};
use crate::runtime::shared::loop_counter::LoopCounter;
use crate::runtime::shared::press_latency_counter::PressLatencyCounter;
use crate::runtime::shared::sleep::SleepCountdown;
#[cfg(feature = "serial")]
use core::fmt::Write;
use rp2040_hal::multicore::Multicore;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::Timer;
use usb_device::bus::UsbBusAllocator;

static mut CORE_1_STACK_AREA: [usize; 2048] = [0; 2048];
// Boot loop if the queue is bugged, not bothering with MemUninit here
#[inline(never)]
#[allow(unused_variables, clippy::needless_pass_by_value)]
pub fn run_right<'a>(
    mc: &'a mut Multicore<'a>,
    usb_bus: UsbBusAllocator<rp2040_hal::usb::UsbBus>,
    mut oled_handle: OledHandle,
    uart_driver: crate::keyboard::split_serial::UartRight,
    right_buttons: RightButtons,
    #[allow(unused_variables, unused_mut)] mut power_led_pin: PowerLed,
    timer: Timer,
) -> ! {
    #[cfg(feature = "serial")]
    unsafe {
        crate::runtime::shared::usb::init_usb(usb_bus);
    };
    let cores = mc.cores();
    let c1 = &mut cores[1];
    let serializer = MessageSerializer::new(uart_driver);
    let (producer, consumer) = new_shared_queue();
    #[allow(static_mut_refs)]
    if let Err(e) = c1.spawn(unsafe { &mut CORE_1_STACK_AREA }, move || {
        run_core1(serializer, right_buttons, timer, producer)
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
    let mut oled = RightOledDrawer::new(oled_handle);
    #[cfg(feature = "serial")]
    let mut last_chars = [0u8; 128];
    #[cfg(feature = "serial")]
    let mut output_all = false;
    #[cfg(feature = "serial")]
    let mut has_dumped = false;
    let mut sleep = SleepCountdown::new();
    let mut press_counter = PressLatencyCounter::new();
    let mut tx: u16 = 0;
    let mut last_avail = 0;
    loop {
        let now = timer.get_counter();
        let avail = consumer.available();
        match pop_message(&consumer) {
            Some(KeycoreToAdminMessage::Loop(lc)) => {
                if sleep.is_awake() {
                    oled.update_scan_loop(lc.as_micros_fraction());
                }
            }
            Some(KeycoreToAdminMessage::Touch {
                tx_bytes,
                loop_duration,
            }) => {
                sleep.touch(now);
                tx += tx_bytes;
                if tx > 9999 {
                    tx = tx_bytes;
                }
                oled.update_touch(tx, press_counter.increment_get_avg(loop_duration));
                oled.show();
            }
            Some(KeycoreToAdminMessage::Reboot) => {
                oled.render_boot_msg();
                reset_to_usb_boot(0, 0);
                panic!("HALT POST RESET");
            }
            _ => {}
        }
        if avail != last_avail {
            oled.update_queue(avail);
            last_avail = avail;
        }
        if sleep.should_sleep(now) {
            sleep.set_sleeping();
            oled.hide();
        }
        oled.render();
        #[cfg(feature = "serial")]
        {
            handle_usb(&mut power_led_pin, &mut last_chars, &mut output_all);
            if output_all && !has_dumped {
                let _ =
                    crate::runtime::shared::usb::acquire_usb().write_str("Right side running\r\n");
                has_dumped = true;
            }
        }
    }
}

#[cfg(feature = "serial")]
fn handle_usb(power_led: &mut PowerLed, last_chars: &mut [u8], output_all: &mut bool) {
    let mut usb = crate::runtime::shared::usb::acquire_usb();
    if usb
        .dev
        .as_mut()
        .unwrap()
        .inner
        .poll(&mut [&mut usb.serial.as_mut().unwrap().inner])
    {
        let last_chars_len = last_chars.len();
        let mut buf = [0u8; 64];
        match usb.serial.as_mut().unwrap().inner.read(&mut buf) {
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
                        let _ = usb.write_str("BOOT\r\n");
                        rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
                    } else if last_chars.ends_with(b"output") {
                        *usb.output = true;
                        let _ = usb.write_str("output ON\r\n");
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

#[allow(clippy::needless_pass_by_value)]
fn run_core1(
    mut serializer: MessageSerializer,
    mut right_buttons: RightButtons,
    timer: Timer,
    producer: Producer,
) -> ! {
    let mut loop_count: LoopCounter<10_000> = LoopCounter::new(timer.get_counter());
    right_buttons.scan_encoder(&mut serializer);
    let mut tx = 0;
    loop {
        let loop_timer = timer.get_counter();
        tx += right_buttons.scan_matrix(&mut serializer, timer, &producer);
        if right_buttons.scan_encoder(&mut serializer) {
            tx += 1;
        }

        if loop_count.increment() {
            let now = timer.get_counter();
            let lc = loop_count.value(now);
            if push_loop_to_admin(&producer, lc) {
                loop_count.reset(now);
            }
        }
        if tx > 0 {
            if let Some(dur) = timer.get_counter().checked_duration_since(loop_timer) {
                if try_push_touch(&producer, tx, dur) {
                    tx = 0;
                }
            }
        }
    }
}
