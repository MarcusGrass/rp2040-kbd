use crate::keyboard::oled::right::RightOledDrawer;
use crate::keyboard::oled::OledHandle;
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::right::message_serializer::MessageSerializer;
use crate::keyboard::right::RightButtons;
use crate::keyboard::usb_serial::{UsbSerial, UsbSerialDevice};
use crate::runtime::shared::cores_right::{
    pop_message, push_loop_to_admin, push_touch_to_admin, KeycoreToAdminMessage,
};
use crate::runtime::shared::loop_counter::LoopCounter;
use crate::runtime::shared::sleep::SleepCountdown;
use crate::runtime::shared::{acquire_matrix_scan, try_acquire_matrix_scan};
use core::fmt::Write;
use embedded_hal::timer::CountDown;
use embedded_io::Read;
use heapless::String;
use liatris::hal;
use nb::block;
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_hal::multicore::Multicore;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::Timer;
use usb_device::bus::{UsbBus, UsbBusAllocator};
use usb_device::device::UsbDevice;

static mut CORE_1_STACK_AREA: [usize; 1024] = [0; 1024];

#[inline(never)]
pub fn run_right<'a>(
    mc: &'a mut Multicore<'a>,
    mut usb_bus: UsbBusAllocator<rp2040_hal::usb::UsbBus>,
    mut oled_handle: OledHandle,
    uart_driver: crate::keyboard::split_serial::UartRight,
    mut right_buttons: RightButtons,
    mut power_led_pin: PowerLed,
    timer: Timer,
) -> ! {
    #[cfg(feature = "serial")]
    unsafe {
        crate::runtime::shared::usb::init_usb(usb_bus)
    };
    let mut oled = RightOledDrawer::new(oled_handle);
    let cores = mc.cores();
    let c1 = &mut cores[1];
    let mut serializer = MessageSerializer::new(uart_driver);
    c1.spawn(unsafe { &mut CORE_1_STACK_AREA }, move || {
        run_core1(serializer, right_buttons, timer)
    })
    .unwrap();
    let mut last_chars = [0u8; 128];
    let mut output_all = false;
    let mut has_dumped = false;
    let mut sleep = SleepCountdown::new();
    loop {
        let now = timer.get_counter();
        match pop_message() {
            Some(KeycoreToAdminMessage::Touch) => {
                sleep.touch(now);
                oled.show();
            }
            Some(KeycoreToAdminMessage::Loop(lc)) => {
                let loop_millis = lc.count as u64 / lc.duration.to_millis();
                if sleep.is_awake() {
                    if let Some((header, body)) = lc.as_display() {
                        oled.update_scan_loop(header, body);
                    }
                }
            }
            _ => {}
        }
        if sleep.should_sleep(now) {
            oled.hide();
        }
        oled.render();
        #[cfg(feature = "serial")]
        {
            handle_usb(&mut power_led_pin, &mut last_chars, &mut output_all);
            if output_all {
                if !has_dumped {
                    let _ = crate::runtime::shared::usb::acquire_usb()
                        .write_str("Right side running\r\n");
                    has_dumped = true;
                }
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
                        reset_to_usb_boot(0, 0);
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

fn run_core1(
    mut serializer: MessageSerializer,
    mut right_buttons: RightButtons,
    mut timer: Timer,
) -> ! {
    let mut loop_count: LoopCounter<100_000> = LoopCounter::new(timer.get_counter());
    right_buttons.scan_encoder(&mut serializer);
    loop {
        if right_buttons.scan_matrix(&mut serializer, timer) {
            push_touch_to_admin();
        }
        if right_buttons.scan_encoder(&mut serializer) {
            push_touch_to_admin();
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
