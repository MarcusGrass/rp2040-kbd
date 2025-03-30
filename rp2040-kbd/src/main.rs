#![cfg_attr(not(test), no_std)]
#![no_main]

mod hid;
pub(crate) mod keyboard;
#[cfg(feature = "left")]
mod keymap;
pub(crate) mod runtime;
mod timer;

use core::ops::Div;
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

use crate::keyboard::oled::OledHandle;
use crate::keyboard::power_led::PowerLed;
use embedded_hal::digital::InputPin;
use liatris::pac::vreg_and_chip_reset::vreg::VSEL_A;
use liatris::pac::I2C1;
use rp2040_hal::clocks::{ClocksManager, PeripheralClock};
use rp2040_hal::fugit::{HertzU32, RateExtU32};
use rp2040_hal::gpio::bank0::{Gpio2, Gpio3};
use rp2040_hal::gpio::{FunctionI2C, Pin, PullDown};
use rp2040_hal::multicore::Multicore;
use rp2040_hal::pll::common_configs::PLL_USB_48MHZ;
use rp2040_hal::pll::{setup_pll_blocking, PLLConfig};
use rp2040_hal::vreg::{get_voltage, set_voltage};
use rp2040_hal::xosc::setup_xosc_blocking;
use rp2040_hal::Clock;
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

/// FREF is the xosc crystal freq (12Mhz)
/// POSTDIV(both) 1 -7
/// If postdiv has different values, POSTDIV1 should be higher
/// for energy efficiency
/// Refdiv recommended to be 1
/// MAX VCO = 1600Mhz
/// Docs says FBDIV which is, but that can be replaced with VCO since it's a part of it
/// Calculate by (FREF / REFDIV) * FBDIV / (POSTDIV1 * POSTDIV2)
/// Where VCO = FREF * FBDIV => FBDIV = VCO / FREF
/// Ex: (12 / 1) * 133 / (6 * 2) => VCO = 12 * 133 = 1596Mhz
/// There's a script for finding your optimal clock frequency here:
/// <https://github.com/raspberrypi/pico-sdk/blob/master/src/rp2_common/hardware_clocks/scripts/vcocalc.py>
/// Ex legal config for 200Mhz
/// `VCO_FREQ` = 1200Mhz
/// refdiv = 1
/// postdiv1 = 6, postdiv2 = 1
/// Higher VCO-freq decreases jitter but increases power consumption,
/// The only way to increase VCO is to compromize on output frequency
/// Below is a Legal config for 199.5Mhz
/// `VCO_FREQ` = 1596
/// REFDIV = 1
/// FBDIV = 133
/// POSTDIV1 = 4
/// POSTDIV2 = 2
const PLL_1995_MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1596),
    refdiv: 1,
    post_div1: 4,
    post_div2: 2,
};

/*/// Example 133Mhz config
const PLL_133_MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1596),
    refdiv: 1,
    post_div1: 6,
    post_div2: 2,
};*/

/// Entry point to our bare-metal application.
///
/// The `#[entry]` macro ensures the Cortex-M start-up code calls this function
/// as soon as all global variables are initialised.
///
/// The function configures the RP2040 peripherals, then echoes any characters
/// received over USB Serial.
#[entry]
fn main() -> ! {
    setup_kbd()
}

#[expect(clippy::too_many_lines, clippy::cast_possible_truncation)]
fn setup_kbd() -> ! {
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();

    // Up voltage if necessary for 200Mhz clock
    if get_voltage(&pac.VREG_AND_CHIP_RESET) != Some(VSEL_A::VOLTAGE1_15) {
        set_voltage(&mut pac.VREG_AND_CHIP_RESET, VSEL_A::VOLTAGE1_15);
    }

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let xosc = setup_xosc_blocking(pac.XOSC, liatris::XOSC_CRYSTAL_FREQ.Hz()).unwrap();
    watchdog.enable_tick_generation((liatris::XOSC_CRYSTAL_FREQ / 1_000_000) as u8);

    let mut clocks = ClocksManager::new(pac.CLOCKS);
    let pll_sys = setup_pll_blocking(
        pac.PLL_SYS,
        xosc.operating_frequency(),
        PLL_1995_MHZ,
        &mut clocks,
        &mut pac.RESETS,
    )
    .unwrap();
    let pll_usb = setup_pll_blocking(
        pac.PLL_USB,
        xosc.operating_frequency(),
        PLL_USB_48MHZ,
        &mut clocks,
        &mut pac.RESETS,
    )
    .unwrap();
    clocks.init_default(&xosc, &pll_sys, &pll_usb).unwrap();
    // I want this high, but also as a clean divisor of the system clock
    // I'm using some weird protocol now with a header and footer on the message,
    // stretching it to 16 bits, to account for connects/disconnects of the right side
    // producing faulty messages.
    // The 'fake-uart' clock divisor is the baud-rate * 16, and can at most be 1,
    // I'm putting it at 1, so that's `199_500_000 / 16 => 12_468_750`.
    // Todo: Maybe check that this is a clean divisor
    let uart_baud = clocks.system_clock.freq().div(16);

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
    let mut oled = setup_oled(
        pac.I2C1,
        &mut pac.RESETS,
        sda_pin,
        scl_pin,
        &clocks.peripheral_clock,
    );

    // Set up the USB driver
    #[cfg(any(feature = "serial", feature = "left"))]
    let usb_bus = usb_device::bus::UsbBusAllocator::new(hal::usb::UsbBus::new(
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
    if is_left {
        #[cfg(feature = "left")]
        {
            // Left side flips tx/rx, check qmk for proton-c in kyria for reference
            let uart = keyboard::split_serial::UartLeft::new(
                pins.gpio1,
                uart_baud,
                clocks.system_clock.freq(),
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
            runtime::left::run_left(
                &mut mc,
                usb_bus,
                oled,
                uart,
                left,
                pl,
                timer,
                &clocks.system_clock,
            );
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
                clocks.system_clock.freq(),
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
            runtime::right::run_right(
                &mut mc,
                #[cfg(feature = "serial")]
                usb_bus,
                oled,
                uart,
                right,
                pl,
                timer,
                &clocks.system_clock,
            );
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

fn setup_oled(
    i2c: I2C1,
    r: &mut pac::RESETS,
    sda: Pin<Gpio2, FunctionI2C, PullDown>,
    scl: Pin<Gpio3, FunctionI2C, PullDown>,
    clock: &PeripheralClock,
) -> OledHandle {
    let i2c = hal::I2C::i2c1(
        i2c,
        sda.reconfigure(),
        scl.reconfigure(),
        400.kHz(),
        r,
        clock,
    );

    let interface = ssd1306::I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate90)
        .into_buffered_graphics_mode();
    display.init().unwrap();
    let _ = display.clear(BinaryColor::Off);
    let _ = display.flush();
    OledHandle::new(display)
}

#[panic_handler]
#[inline(never)]
fn halt(_info: &core::panic::PanicInfo) -> ! {
    loop {
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}
