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

#![no_std]
#![no_main]

mod debugger;
mod lock;

use core::borrow::BorrowMut;
use core::cell::UnsafeCell;
use core::convert::Infallible;
// The macro for our start-up function
use elite_pi::{entry, Pins};

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

// A shorter alias for the Peripheral Access Crate, which provides low-level
// register access
use elite_pi::hal::pac;

// A shorter alias for the Hardware Abstraction Layer, which provides
// higher-level drivers.
use elite_pi::hal;

// USB Device support
use usb_device::{class_prelude::*, prelude::*};

// USB Communications Class Device support
use usbd_serial::SerialPort;

use crate::debugger::DebugBuffer;
use crate::lock::SpinLockN;
use core::fmt::Write;
use core::sync::atomic::{AtomicU32, Ordering};
use elite_pi::pac::UART0;
use embedded_hal::digital::v2::{InputPin, PinState};
use embedded_hal::prelude::_embedded_hal_blocking_delay_DelayMs;
use heapless::String;
use rp2040_hal::fugit::RateExtU32;
use rp2040_hal::gpio::bank0::{
    Gpio19, Gpio20, Gpio21, Gpio22, Gpio23, Gpio26, Gpio27, Gpio28, Gpio29, Gpio6, Gpio7, Gpio8,
    Gpio9,
};
use rp2040_hal::gpio::{
    AsInputPin, FunctionNull, FunctionSio, Pin, PinId, PullDown, PullUp, SioInput,
};
use rp2040_hal::pio::PIOExt;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::sio::{Spinlock, Spinlock0};
use rp2040_hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use rp2040_hal::{Clock, Timer};

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

    let mut timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let sio = hal::Sio::new(pac.SIO);
    let mut pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let uart_pins = (
        // UART TX (characters sent from RP2040) on pin 1 (GPIO0)
        pins.gpio0.into_function::<hal::gpio::FunctionUart>(),
        // UART RX (characters received by RP2040) on pin 2 (GPIO1)
        pins.gpio1.into_function::<hal::gpio::FunctionUart>(),
    );

    let _uart = hal::uart::UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(115_200.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);

    let _i2c_pio = i2c_pio::I2C::new(
        &mut pio,
        pins.gpio2,
        pins.gpio3,
        sm0,
        100.kHz(),
        clocks.system_clock.freq(),
    );

    let mut prev = timer.get_counter();

    // Set up the USB driver
    let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut read = [0u8; 1024];
    let mut write = [0u8; 2048];
    // Set up the USB Communications Class Device driver
    let mut serial = SerialPort::new_with_store(&usb_bus, read, write);
    let mut btns = ButtonPins::new(
        (
            pins.gpio29.into_pull_up_input(),
            pins.gpio27.into_pull_up_input(),
            pins.gpio6.into_pull_up_input(),
            pins.gpio7.into_pull_up_input(),
            pins.gpio8.into_pull_up_input(),
        ),
        (
            pins.gpio9.into_pull_down_input(),
            pins.gpio26.into_pull_up_input(),
            pins.gpio22.into_pull_down_input(),
            pins.gpio20.into_pull_down_input(),
            pins.gpio23.into_pull_down_input(),
            pins.gpio21.into_pull_down_input(),
        ),
    );
    btns.init();
    // Encoder
    pins.gpio5.into_pull_up_input();
    pins.gpio4.into_pull_down_input();

    /*
    pins.gpio0.set_input_enable(true);
    pins.gpio1.set_input_enable(true);
    pins.gpio2.set_input_enable(true);
    pins.gpio3.set_input_enable(true);
    pins.gpio4.set_input_enable(true);
    pins.gpio5.set_input_enable(true);
    pins.gpio6.set_input_enable(true);
    pins.gpio7.set_input_enable(true);
    pins.gpio8.set_input_enable(true);
    pins.gpio9.set_input_enable(true);
    pins.gpio10.set_input_enable(true);
    pins.gpio11.set_input_enable(true);
    pins.gpio12.set_input_enable(true);
    pins.gpio13.set_input_enable(true);
    pins.gpio14.set_input_enable(true);
    pins.gpio15.set_input_enable(true);
    pins.gpio16.set_input_enable(true);
    pins.gpio17.set_input_enable(true);
    pins.gpio18.set_input_enable(true);
    pins.gpio19.set_input_enable(true);
    pins.gpio20.set_input_enable(true);
    pins.gpio21.set_input_enable(true);
    pins.gpio22.set_input_enable(true);
    pins.gpio26.set_input_enable(true);
    pins.gpio27.set_input_enable(true);
    pins.gpio28.set_input_enable(true);
    let mut pin_state: [(bool, bool, bool); 26] = [(false, false, false); 26];

    let mut p29 = pins.gpio29.into_pull_up_input();
    p29.set_input_enable(true);
    let mut p27 = pins.gpio27.into_pull_up_input();
    p27.set_input_enable(true);
    let mut p9 = pins.gpio9.into_pull_down_input();
    p9.set_input_enable(true);

     */

    // Create a USB device with a fake VID and PID
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake company")
        .product("Serial port")
        .serial_number("TEST")
        .device_class(2) // from: https://www.usb.org/defined-class-codes
        .build();

    let mut last_chars = [0u8; 128];
    let mut output_all = false;
    loop {
        let now = timer.get_counter();
        if let Some(dur) = now.checked_duration_since(prev) {
            if dur.to_millis() > 200 {
                /*
                if let (Ok(lo), Ok(hi)) = (p29.is_low(), p29.is_high()) {
                    let mut s: String<128> = String::new();
                    let _ = s.write_fmt(format_args!("gpio29: lo={lo}, hi={hi}\r\n"));
                    serial_write_all(&mut serial, s.as_bytes(), &mut timer);
                }
                if let (Ok(lo), Ok(hi)) = (p27.is_low(), p27.is_high()) {
                    let mut s: String<128> = String::new();
                    let _ = s.write_fmt(format_args!("gpio27: lo={lo}, hi={hi}\r\n"));
                    serial_write_all(&mut serial, s.as_bytes(), &mut timer);
                }

                if let (Ok(lo), Ok(hi)) = (p9.is_low(), p9.is_high()) {
                    let mut s: String<128> = String::new();
                    let _ = s.write_fmt(format_args!("gpio9: lo={lo}, hi={hi}\r\n"));
                    serial_write_all(&mut serial, s.as_bytes(), &mut timer);
                }

                 */

                //let _ = dbg.write_str("Ping\r\n");

                prev = now;
            }
        }
        handle_usb(&mut usb_dev, &mut serial, &mut last_chars, &mut output_all);
        if output_all {
            check_matrix(&btns, &mut serial, &mut timer);
        }
    }
}

type RowPin<Id> = Pin<Id, FunctionSio<SioInput>, PullUp>;
type ColPin<Id> = Pin<Id, FunctionSio<SioInput>, PullDown>;
// Left side
struct ButtonPins {
    rows: (
        RowPin<Gpio29>,
        RowPin<Gpio27>,
        RowPin<Gpio6>,
        RowPin<Gpio7>,
        RowPin<Gpio8>,
    ),
    cols: (
        ColPin<Gpio9>,
        RowPin<Gpio26>,
        ColPin<Gpio22>,
        ColPin<Gpio20>,
        ColPin<Gpio23>,
        ColPin<Gpio21>,
    ),
}

impl ButtonPins {
    pub fn init(&mut self) {
        self.rows.0.set_input_enable(true);
        self.rows.1.set_input_enable(true);
        self.rows.2.set_input_enable(true);
        self.rows.3.set_input_enable(true);
        self.rows.4.set_input_enable(true);
        self.cols.0.set_input_enable(true);
        self.cols.1.set_input_enable(true);
        self.cols.2.set_input_enable(true);
        self.cols.3.set_input_enable(true);
        self.cols.4.set_input_enable(true);
        self.cols.5.set_input_enable(true);
    }
    pub fn matrix(&self) -> Matrix {
        Matrix {
            rows: [
                (
                    self.rows.0.is_low().unwrap(),
                    self.rows.0.is_high().unwrap(),
                ),
                (
                    self.rows.1.is_low().unwrap(),
                    self.rows.1.is_high().unwrap(),
                ),
                (
                    self.rows.2.is_low().unwrap(),
                    self.rows.2.is_high().unwrap(),
                ),
                (
                    self.rows.3.is_low().unwrap(),
                    self.rows.3.is_high().unwrap(),
                ),
                (
                    self.rows.4.is_low().unwrap(),
                    self.rows.4.is_high().unwrap(),
                ),
            ],
            cols: [
                (
                    self.cols.0.is_low().unwrap(),
                    self.cols.0.is_high().unwrap(),
                ),
                (
                    self.cols.1.is_low().unwrap(),
                    self.cols.1.is_high().unwrap(),
                ),
                (
                    self.cols.2.is_low().unwrap(),
                    self.cols.2.is_high().unwrap(),
                ),
                (
                    self.cols.3.is_low().unwrap(),
                    self.cols.3.is_high().unwrap(),
                ),
                (
                    self.cols.4.is_low().unwrap(),
                    self.cols.4.is_high().unwrap(),
                ),
                (
                    self.cols.5.is_low().unwrap(),
                    self.cols.5.is_high().unwrap(),
                ),
            ],
        }
    }

    pub fn new(
        rows: (
            RowPin<Gpio29>,
            RowPin<Gpio27>,
            RowPin<Gpio6>,
            RowPin<Gpio7>,
            RowPin<Gpio8>,
        ),
        cols: (
            ColPin<Gpio9>,
            RowPin<Gpio26>,
            ColPin<Gpio22>,
            ColPin<Gpio20>,
            ColPin<Gpio23>,
            ColPin<Gpio21>,
        ),
    ) -> Self {
        Self { rows, cols }
    }
}

fn handle_usb<B: UsbBus, B1: BorrowMut<[u8]>, B2: BorrowMut<[u8]>>(
    usb_dev: &mut UsbDevice<B>,
    serial: &mut SerialPort<B, B1, B2>,
    last_chars: &mut [u8],
    output_all: &mut bool,
) {
    if usb_dev.poll(&mut [serial]) {
        let last_chars_len = last_chars.len();
        let mut buf = [0u8; 64];
        match serial.read(&mut buf) {
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
                        *output_all = true;
                    }
                }
            }
        }
    }
}

fn serial_write_all<W: UsbBus, B1: BorrowMut<[u8]>, B2: BorrowMut<[u8]>>(
    serial: &mut SerialPort<W, B1, B2>,
    buf: &[u8],
    timer: &mut Timer,
) {
    for chunk in buf.chunks(16) {
        let mut rem = chunk;
        loop {
            if rem.is_empty() {
                break;
            }
            let res = serial.write(rem);
            match res {
                Ok(wrote) => {
                    rem = &rem[wrote..];
                }
                Err(UsbError::WouldBlock) => {
                    timer.delay_ms(2);
                    continue;
                }
                Err(e) => {
                    let mut buf: String<128> = String::new();
                    let _ = buf.write_fmt(format_args!("Write err: {e:?}\r\n"));
                    serial_write_all(serial, buf.as_bytes(), timer);
                    return;
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct Slot {
    index: u8,
    on: bool,
}

impl Slot {
    const fn new(index: u8, on: bool) -> Self {
        Self { index, on }
    }
}

#[derive(Copy, Clone, Debug)]
struct Matrix {
    rows: [(bool, bool); 5],
    cols: [(bool, bool); 6],
}

static MATRIX: SpinLockN<Matrix> = SpinLockN::new(Matrix {
    rows: [
        (false, false),
        (false, false),
        (false, false),
        (false, false),
        (false, false),
    ],
    cols: [
        (false, false),
        (false, false),
        (false, false),
        (false, false),
        (false, false),
        (false, false),
    ],
});

fn check_all_pins<W: UsbBus, B1: BorrowMut<[u8]>, B2: BorrowMut<[u8]>>(
    pins: &Pins,
    pin_state: &mut [(bool, bool, bool); 23],
    serial: &mut SerialPort<W, B1, B2>,
    timer: &mut Timer,
) {
    let cur_state = [
        get_state(pins.gpio0.as_input()),
        get_state(pins.gpio1.as_input()),
        get_state(pins.gpio2.as_input()),
        get_state(pins.gpio3.as_input()),
        get_state(pins.gpio4.as_input()),
        get_state(pins.gpio5.as_input()),
        get_state(pins.gpio6.as_input()),
        get_state(pins.gpio7.as_input()),
        get_state(pins.gpio8.as_input()),
        get_state(pins.gpio9.as_input()),
        get_state(pins.gpio10.as_input()),
        get_state(pins.gpio11.as_input()),
        get_state(pins.gpio12.as_input()),
        get_state(pins.gpio13.as_input()),
        get_state(pins.gpio14.as_input()),
        get_state(pins.gpio15.as_input()),
        get_state(pins.gpio16.as_input()),
        get_state(pins.gpio20.as_input()),
        get_state(pins.gpio21.as_input()),
        get_state(pins.gpio22.as_input()),
        get_state(pins.gpio26.as_input()),
        get_state(pins.gpio27.as_input()),
        get_state(pins.gpio28.as_input()),
    ];
    for (ind, (old, new)) in (pin_state.iter_mut().zip(cur_state.iter())).enumerate() {
        if old != new {
            let mut bytes: String<128> = String::new();
            let _ = bytes.write_fmt(format_args!(
                "Diff on {ind} ({}, {}, {}) -> ({}, {}, {})\r\n",
                old.0, old.1, old.2, new.0, new.1, new.2
            ));
            serial_write_all(serial, bytes.as_bytes(), timer);
            *old = *new;
        }
    }
}

fn get_state<I: PinId>(pin: AsInputPin<I, FunctionNull, PullDown>) -> (bool, bool, bool) {
    match (pin.is_low(), pin.is_high()) {
        (Ok(a), Ok(b)) => (a, b, false),
        _ => (false, false, true),
    }
}

fn check_matrix<W: UsbBus, B1: BorrowMut<[u8]>, B2: BorrowMut<[u8]>>(
    pins: &ButtonPins,
    serial_port: &mut SerialPort<W, B1, B2>,
    timer: &mut Timer,
) {
    let new = pins.matrix();
    let old = MATRIX.lock_mutex();
    //let mut cur_state: String<512> = String::new();
    for (ind, (old_row, new_row)) in old.value.rows.iter().zip(new.rows.iter()).enumerate() {
        //let _ = cur_state.write_fmt(format_args!("r{ind}:[{},{}]), ", new_row.0 as u8, new_row.1 as u8));
        if old_row != new_row {
            let mut s: String<128> = String::new();
            let _ = s.write_fmt(format_args!(
                "Diff on r{}[{},{}]->[{},{}]\r\n",
                ind, old_row.0 as u8, old_row.1 as u8, new_row.0 as u8, new_row.1 as u8
            ));
            serial_write_all(serial_port, s.as_bytes(), timer);
        }
    }
    for (ind, (old_col, new_col)) in old.value.cols.iter().zip(new.cols.iter()).enumerate() {
        //let _ = cur_state.write_fmt(format_args!("c{ind}:[{},{}]), ", new_col.0 as u8, new_col.1 as u8));
        if old_col != new_col {
            let mut s: String<128> = String::new();
            let _ = s.write_fmt(format_args!(
                "Diff on c{}[{},{}]->[{},{}]\r\n",
                ind, old_col.0 as u8, old_col.1 as u8, new_col.0 as u8, new_col.1 as u8
            ));
            serial_write_all(serial_port, s.as_bytes(), timer);
        }
    }
    /*
    cur_state.pop();
    cur_state.pop();
    let _ = cur_state.push_str("\r\n");
    serial_write_all(serial_port, cur_state.as_bytes(), timer);

     */
    *old.value = new;
}
