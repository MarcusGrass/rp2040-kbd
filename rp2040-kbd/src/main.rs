//! # Pico USB Serial Example
//!
//! Creates a USB Serial device on a Pico board, with the USB driver running in
//! the main thread.
//!
//! This will create a USB Serial device echoing anything it receives. Incoming
//! ASCII characters are converted to upercase, so you can tell it is working
//! and not just local-echo!
//!
//! See the `Cargo.toml` file for Copyright and license details.
//!
#![cfg_attr(not(test), no_std)]
//#![no_std]
#![no_main]

mod hid;
pub(crate) mod keyboard;
#[cfg(feature = "left")]
mod keymap;
pub(crate) mod runtime;
mod timer;

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::pixelcolor::BinaryColor;
// The macro for our start-up function
use liatris::{entry, Pins};

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
#[cfg(not(test))]
// A shorter alias for the Peripheral Access Crate, which provides low-level
// register access
use liatris::hal::pac;

// A shorter alias for the Hardware Abstraction Layer, which provides
// higher-level drivers.
use liatris::hal;

// USB Device support
use usb_device::class_prelude::*;

use crate::keyboard::oled::OledHandle;
use crate::keyboard::power_led::PowerLed;
use embedded_hal::digital::InputPin;
use rp2040_hal::fugit::RateExtU32;
use rp2040_hal::multicore::Multicore;
use ssd1306::mode::DisplayConfig;
use ssd1306::prelude::DisplayRotation;
use ssd1306::size::DisplaySize128x32;
use ssd1306::Ssd1306;

#[cfg(all(feature = "serial", feature = "hiddev"))]
const _ILLEGAL_FEATURES: () = assert!(false, "Can't compile as both serial and hiddev");

#[cfg(all(feature = "left", feature = "right"))]
const _ILLEGAL_SIDES: () = assert!(false, "Can't compile as both right and left");

#[cfg(all(feature = "hiddev", feature = "right"))]
const _RIGHT_HIDDEN: () = assert!(false, "Can't compile right as hiddev");

/// Entry point to our bare-metal application.
///
/// The `#[entry]` macro ensures the Cortex-M start-up code calls this function
/// as soon as all global variables are initialised.
///
/// The function configures the RP2040 peripherals, then echoes any characters
/// received over USB Serial.
#[entry]
#[allow(clippy::too_many_lines)]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    //
    // The default is to generate a 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
        liatris::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let mut sio = hal::Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let sda_pin = pins.gpio2.into_function::<hal::gpio::FunctionI2C>();
    let scl_pin = pins.gpio3.into_function::<hal::gpio::FunctionI2C>();

    let i2c = hal::I2C::i2c1(
        pac.I2C1,
        sda_pin.reconfigure(),
        scl_pin.reconfigure(),
        400.kHz(),
        &mut pac.RESETS,
        &clocks.peripheral_clock,
    );

    let interface = ssd1306::I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate90)
        .into_buffered_graphics_mode();
    display.init().unwrap();
    let _ = display.clear(BinaryColor::Off);
    let _ = display.flush();
    let mut oled = OledHandle::new(display);

    // Set up the USB driver
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut side_check_pin = pins.gpio28.into_pull_up_input();

    let power_led_pin = pins.power_led.into_push_pull_output();
    let pl = PowerLed::new(power_led_pin);
    let is_left = side_check_pin.is_high().unwrap();
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    // I want this high, but also as a clean divisor of the system clock
    // I'm using some weird protocol now with a header and footer on the message,
    // stretching it to 16 bits, to account for connects/disconnects of the right side
    // producing faulty messages.
    // The 'fake-uart' clock divisor is the baud-rate * 16, and can at most be 1,
    // I'm putting it at 1, so that's 125_000_000 / 16 => 7_812_500.
    // That's going to be 125_000_000 / 8 = 15 625 000 bits per second
    // or 1 925 125 bytes per second
    // which is 512 nanos per byte, two bytes per message is around 1 microsecond per message
    // of latency. Clock speed is 8 nanos per cycle, each bit is therefore held for 64 nanos (8 cycles),
    // each byte is transferred in 64 * 8 = 512 nanos, which adds up with the above.
    let uart_baud = 7_812_500.Hz();
    if is_left {
        #[cfg(feature = "left")]
        {
            // Left side flips tx/rx, check qmk for proton-c in kyria for reference
            let uart = keyboard::split_serial::UartLeft::new(
                pins.gpio1,
                uart_baud,
                125.MHz(),
                pac.PIO0,
                &mut pac.RESETS,
            );
            let left = crate::keyboard::left::LeftButtons::new(
                (
                    pins.gpio29.into_pull_up_input(),
                    pins.gpio27.into_pull_up_input(),
                    pins.gpio6.into_pull_up_input(),
                    pins.gpio7.into_pull_up_input(),
                    pins.gpio8.into_pull_up_input(),
                ),
                (
                    Some(pins.gpio9.into_pull_up_input()),
                    Some(pins.gpio26.into_pull_up_input()),
                    Some(pins.gpio22.into_pull_up_input()),
                    Some(pins.gpio20.into_pull_up_input()),
                    Some(pins.gpio23.into_pull_up_input()),
                    Some(pins.gpio21.into_pull_up_input()),
                ),
            );
            runtime::left::run_left(&mut mc, usb_bus, oled, uart, left, pl, timer);
        }
        #[cfg(not(feature = "left"))]
        {
            // Hard error, needs new firmware loaded
            oled.write_bad_boot_msg();
            rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
            unreachable!("Should have gone into boot");
        }
    } else {
        #[cfg(feature = "right")]
        {
            let uart = keyboard::split_serial::UartRight::new(
                pins.gpio1.reconfigure(),
                uart_baud,
                125.MHz(),
                pac.PIO0,
                &mut pac.RESETS,
            );
            let right = crate::keyboard::right::RightButtons::new(
                (
                    pins.gpio29.into_pull_up_input(),
                    pins.gpio4.into_pull_up_input(),
                    pins.gpio20.into_pull_up_input(),
                    pins.gpio23.into_pull_up_input(),
                    pins.gpio21.into_pull_up_input(),
                ),
                (
                    pins.gpio22.into_pull_up_input(),
                    pins.gpio5.into_pull_up_input(),
                    pins.gpio6.into_pull_up_input(),
                    pins.gpio7.into_pull_up_input(),
                    pins.gpio8.into_pull_up_input(),
                    pins.gpio9.into_pull_up_input(),
                ),
                crate::keyboard::right::RotaryEncoder::new(
                    pins.gpio26.into_pull_up_input(),
                    pins.gpio27.into_pull_up_input(),
                ),
            );
            runtime::right::run_right(&mut mc, usb_bus, oled, uart, right, pl, timer);
        }
        #[cfg(not(feature = "right"))]
        {
            // Hard error, needs new firmware loaded
            oled.write_bad_boot_msg();
            rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
            unreachable!("Should have gone into boot");
        }
    }
}

#[panic_handler]
#[inline(never)]
fn halt(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}
