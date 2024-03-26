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
#![allow(unused_imports)]
#![allow(unused)]
#![allow(dead_code)]

#![cfg_attr(not(test), no_std)]
//#![no_std]

#![no_main]

pub(crate) mod debugger;
pub(crate) mod keyboard;
pub(crate) mod lock;
pub(crate) mod runtime;
pub(crate) mod keycodes;

use core::borrow::BorrowMut;
// The macro for our start-up function
use elite_pi::{entry, Pins};

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
#[cfg(not(test))]
#[allow(unused_imports)]
use panic_halt as _;

// A shorter alias for the Peripheral Access Crate, which provides low-level
// register access
use elite_pi::hal::pac;

// A shorter alias for the Hardware Abstraction Layer, which provides
// higher-level drivers.
use elite_pi::hal;

// USB Device support
use usb_device::{class_prelude::*};

use core::fmt::Write;
use embedded_graphics::Drawable;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use embedded_hal::prelude::_embedded_hal_blocking_delay_DelayMs;
use pio_uart::PioUart;
use rp2040_hal::fugit::RateExtU32;
use rp2040_hal::gpio::{
    PinId
};
use rp2040_hal::pio::PIOExt;
use rp2040_hal::uart::{DataBits, StopBits, UartConfig};
use rp2040_hal::{Clock};
use rp2040_hal::multicore::Multicore;
use ssd1306::mode::DisplayConfig;
use ssd1306::prelude::{DisplayRotation, WriteOnlyDataCommand};
use ssd1306::size::DisplaySize128x32;
use ssd1306::Ssd1306;
use crate::keyboard::left::LeftButtons;
use crate::keyboard::oled::{OledHandle};
use crate::keyboard::power_led::PowerLed;
use crate::keyboard::right::RightButtons;
use crate::keyboard::split_serial::{UartLeft, UartRight};
use crate::keyboard::usb_serial::{UsbSerial, UsbSerialDevice};
use crate::runtime::left::run_left;
use crate::runtime::right::run_right;

/// Entry point to our bare-metal application.
///
/// The `#[entry]` macro ensures the Cortex-M start-up code calls this function
/// as soon as all global variables are initialised.
///
/// The function configures the RP2040 peripherals, then echoes any characters
/// received over USB Serial.
#[entry]
fn main() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    //
    // The default is to generate a 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
        elite_pi::XOSC_CRYSTAL_FREQ,
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
    let mut pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let sda_pin = pins.gpio2.into_function::<hal::gpio::FunctionI2C>();
    let scl_pin = pins.gpio3.into_function::<hal::gpio::FunctionI2C>();

    let i2c = hal::I2C::i2c1(
        pac.I2C1,
        sda_pin,
        scl_pin,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.peripheral_clock,
    );

    let interface = ssd1306::I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate90)
        .into_buffered_graphics_mode();
    display.init().unwrap();
    display.clear();
    let _ = display.flush();
    let oled = OledHandle::new(display);


    // Set up the USB driver
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let side_check_pin = pins.gpio28.as_input();

    let power_led_pin = pins.power_led.into_push_pull_output();
    let pl = PowerLed::new(power_led_pin);
    let is_left = side_check_pin.is_high().unwrap();
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let uart_baud = 115200.Hz();
    if is_left {
        // Left side flips tx/rx, check qmk for proton-c in kyria for reference
        let uart = UartLeft::new(
            pins.gpio1,
            uart_baud,
            125.MHz(),
            pac.PIO0,
            &mut pac.RESETS
        );
        let left = LeftButtons::new(
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
            )
        );
        run_left(usb_bus, oled, uart, left, pl, timer);
    } else {
        let uart = UartRight::new(
            pins.gpio1.reconfigure(),
            uart_baud,
            125.MHz(),
            pac.PIO0,
            &mut pac.RESETS,
        );
        let right = RightButtons::new(
            (
                pins.gpio29.into_pull_up_input(),
                pins.gpio4.into_pull_up_input(),
                pins.gpio20.into_pull_up_input(),
                pins.gpio23.into_pull_up_input(),
                pins.gpio21.into_pull_up_input(),
                ),
            (
                Some(pins.gpio22.into_pull_up_input()),
                Some(pins.gpio5.into_pull_up_input()),
                Some(pins.gpio6.into_pull_up_input()),
                Some(pins.gpio7.into_pull_up_input()),
                Some(pins.gpio8.into_pull_up_input()),
                Some(pins.gpio9.into_pull_up_input()),
            )
        );
        run_right(&mut mc, usb_bus, oled, uart, right, pl, timer);
    }

}