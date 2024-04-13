use crate::keyboard::oled::right::RightOledDrawer;
use crate::keyboard::oled::OledHandle;
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::right::message_serializer::MessageSerializer;
use crate::keyboard::right::RightButtons;
use crate::runtime::shared::cores_right::{
    pop_message, push_loop_to_admin, try_push_tx, KeycoreToAdminMessage, Producer,
};
use crate::runtime::shared::loop_counter::LoopCounter;
use crate::runtime::shared::sleep::SleepCountdown;
#[cfg(feature = "serial")]
use core::fmt::Write;
use core::sync::atomic::AtomicUsize;
use rp2040_hal::multicore::Multicore;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::Timer;
use rp2040_kbd_lib::queue::new_atomic_producer_consumer;
use usb_device::bus::UsbBusAllocator;

static mut CORE_1_STACK_AREA: [usize; 2048] = [0; 2048];
// Boot loop if the queue is bugged, not bothering with MemUninit here
static mut ATOMIC_QUEUE_MEM_AREA: [KeycoreToAdminMessage; 32] = [KeycoreToAdminMessage::Reboot; 32];
static mut ATOMIC_QUEUE_HEAD: AtomicUsize = AtomicUsize::new(0);
static mut ATOMIC_QUEUE_TAIL: AtomicUsize = AtomicUsize::new(0);
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
    #[allow(static_mut_refs)]
    let (producer, consumer) = unsafe {
        new_atomic_producer_consumer(
            &mut ATOMIC_QUEUE_MEM_AREA,
            &mut ATOMIC_QUEUE_HEAD,
            &mut ATOMIC_QUEUE_TAIL,
        )
    };
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
    let mut tx: u16 = 0;
    loop {
        let now = timer.get_counter();
        match pop_message(&consumer) {
            Some(KeycoreToAdminMessage::Loop(lc)) => {
                if sleep.is_awake() {
                    if let Some((header, body)) = lc.as_display() {
                        oled.update_scan_loop(header, body);
                    }
                }
            }
            Some(KeycoreToAdminMessage::Tx(transmitted)) => {
                tx = tx.wrapping_add(transmitted);
                sleep.touch(now);
                oled.update_tx(tx);
                oled.show();
            }
            Some(KeycoreToAdminMessage::Reboot) => {
                oled.render_boot_msg();
                reset_to_usb_boot(0, 0);
                panic!("HALT POST RESET");
            }
            _ => {}
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
        tx += right_buttons.scan_matrix(&mut serializer, timer, &producer);
        if right_buttons.scan_encoder(&mut serializer) {
            tx += 1;
        }
        if tx > 0 && try_push_tx(&producer, tx) {
            tx = 0;
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
