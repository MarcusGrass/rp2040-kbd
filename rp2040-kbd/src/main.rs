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
use rp_pico::{entry, Pins};

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

// A shorter alias for the Peripheral Access Crate, which provides low-level
// register access
use rp_pico::hal::pac;

// A shorter alias for the Hardware Abstraction Layer, which provides
// higher-level drivers.
use rp_pico::hal;

// USB Device support
use usb_device::{class_prelude::*, prelude::*};

// USB Communications Class Device support
use usbd_serial::SerialPort;

use core::fmt::Write;
use core::sync::atomic::{AtomicU32, Ordering};
use embedded_hal::digital::v2::{InputPin, PinState};
use embedded_hal::prelude::_embedded_hal_blocking_delay_DelayMs;
use heapless::String;
use rp2040_hal::gpio::bank0::{Gpio19, Gpio20, Gpio21, Gpio22, Gpio23, Gpio26, Gpio27, Gpio28, Gpio29, Gpio6, Gpio7, Gpio8, Gpio9};
use rp2040_hal::gpio::{AsInputPin, FunctionNull, FunctionSio, Pin, PinId, PullDown, PullUp, SioInput};
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::sio::{Spinlock, Spinlock0};
use rp2040_hal::Timer;
use crate::debugger::{DebugBuffer};
use crate::lock::SpinLockN;

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
        rp_pico::XOSC_CRYSTAL_FREQ,
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
    let mut pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
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
    let mut write = [0u8; 1024];
    // Set up the USB Communications Class Device driver
    let mut serial = SerialPort::new_with_store(&usb_bus, read, write);
    /*
    let mut btns = ButtonPins::new(
        (
            pins.gpio28.into_pull_up_input(),
            pins.gpio27.into_pull_up_input(),
            pins.gpio6.into_pull_up_input(),
            pins.gpio7.into_pull_up_input(),
            pins.gpio8.into_pull_up_input(),
            ),
    (
        pins.gpio9.into_pull_up_input(),
        pins.gpio26.into_pull_up_input(),
        pins.gpio22.into_pull_up_input(),
        pins.gpio20.into_pull_up_input(),
        pins.gpio19.into_pull_up_input(),
        pins.gpio21.into_pull_up_input(),
        ),
    );

     */
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
    let mut dbg = DebugBuffer::new();

    // Create a USB device with a fake VID and PID
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Fake company")
        .product("Serial port")
        .serial_number("TEST")
        .device_class(2) // from: https://www.usb.org/defined-class-codes
        .build();


    let mut last_chars = [0u8; 128];
    let mut output_all = false;
    let _ = dbg.write_str("Starting output runner\r\n");
    loop {
        let now = timer.get_counter();
        if let Some(dur) = now.checked_duration_since(prev) {
            if dur.to_secs() > 2 {
                let _ = dbg.write_str("Ping\r\n");
                prev = now;
            }
        }
        handle_usb(&mut usb_dev, &mut serial, &mut last_chars, &mut output_all);
        //check_matrix(&btns);
        check_all_pins(&pins, &mut pin_state, &mut dbg);
        if output_all {
            handle_output(&mut serial, &mut dbg, &mut timer);
        }
    }
}

type ButtonPin<Id> = Pin<Id, FunctionSio<SioInput>, PullUp>;
// Left side
struct ButtonPins {
    rows: (
        ButtonPin<Gpio28>,
        ButtonPin<Gpio27>,
        ButtonPin<Gpio6>,
        ButtonPin<Gpio7>,
        ButtonPin<Gpio8>,
    ),
    cols: (
        ButtonPin<Gpio9>,
        ButtonPin<Gpio26>,
        ButtonPin<Gpio22>,
        ButtonPin<Gpio20>,
        ButtonPin<Gpio19>,
        ButtonPin<Gpio21>,
    )
}

impl ButtonPins {
    pub fn new(rows: (ButtonPin<Gpio28>, ButtonPin<Gpio27>, ButtonPin<Gpio6>, ButtonPin<Gpio7>, ButtonPin<Gpio8>), cols: (ButtonPin<Gpio9>, ButtonPin<Gpio26>, ButtonPin<Gpio22>, ButtonPin<Gpio20>, ButtonPin<Gpio19>, ButtonPin<Gpio21>)) -> Self {
        Self { rows, cols }
    }

    pub fn matrix(&self) -> Matrix {
        Matrix {
            rows: [
                Slot::new(0, self.rows.0.is_low().unwrap()),
                Slot::new(1, self.rows.1.is_low().unwrap()),
                Slot::new(2, self.rows.2.is_low().unwrap()),
                Slot::new(3, self.rows.3.is_low().unwrap()),
                Slot::new(4, self.rows.4.is_low().unwrap()),
            ],
            cols: [
                Slot::new(0, self.cols.0.is_low().unwrap()),
                Slot::new(1, self.cols.1.is_low().unwrap()),
                Slot::new(2, self.cols.2.is_low().unwrap()),
                Slot::new(3, self.cols.3.is_low().unwrap()),
                Slot::new(4, self.cols.4.is_low().unwrap()),
                Slot::new(5, self.cols.5.is_low().unwrap()),
            ],
        }
    }
}



fn handle_usb<B: UsbBus, B1: BorrowMut<[u8]>, B2: BorrowMut<[u8]>>(usb_dev: &mut UsbDevice<B>, serial: &mut SerialPort<B, B1, B2>, last_chars: &mut [u8], output_all: &mut bool) {
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

fn handle_output<B: UsbBus, B1: BorrowMut<[u8]>, B2: BorrowMut<[u8]>>(serial: &mut SerialPort<B, B1, B2>, dbg: &mut DebugBuffer, timer: &mut Timer) {
    dbg.use_content(|s| {
        if !s.is_empty() {
            serial_write_all(serial, s, timer)
        }
    });
}

fn serial_write_all<W: UsbBus, B1: BorrowMut<[u8]>, B2: BorrowMut<[u8]>>(serial: &mut SerialPort<W, B1, B2>, buf: &[u8], timer: &mut Timer) {
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
    rows: [Slot; 5],
    cols: [Slot; 6],
}



static MATRIX: SpinLockN<Matrix> = SpinLockN::new(
    Matrix {
        rows: [
            Slot::new(0, false), Slot::new(1, false), Slot::new(2, false),
            Slot::new(3, false), Slot::new(4, false),
        ],
        cols: [
            Slot::new(0, false), Slot::new(1, false), Slot::new(2, false),
            Slot::new(3, false), Slot::new(4, false), Slot::new(5, false),
        ],
    }
);

fn check_all_pins(pins: &Pins, pin_state: &mut [(bool, bool, bool); 26], dbg: &mut DebugBuffer) {
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
        get_state(pins.gpio17.as_input()),
        get_state(pins.gpio18.as_input()),
        get_state(pins.gpio19.as_input()),
        get_state(pins.gpio20.as_input()),
        get_state(pins.gpio21.as_input()),
        get_state(pins.gpio22.as_input()),
        get_state(pins.gpio26.as_input()),
        get_state(pins.gpio27.as_input()),
        get_state(pins.gpio28.as_input()),
    ];
    unsafe {
        if *pin_state != cur_state {
            for (ind, (old, new)) in (pin_state.iter().zip(cur_state.iter())).enumerate() {
                if old != new {
                    let _ = dbg.write_fmt(format_args!("Diff on {ind} ({}, {}, {}) -> ({}, {}, {})\r\n", old.0, old.1, old.2, new.0, new.1, new.2));
                }
            }
            *pin_state = cur_state;
        }
    }
}

fn get_state<I: PinId>(pin: AsInputPin<I, FunctionNull, PullDown>) -> (bool, bool, bool) {
    match (pin.is_low(), pin.is_high()) {
        (Ok(a), Ok(b)) => (a, b, false),
        _ => (false, false, true),
    }
}

fn check_matrix(pins: &ButtonPins) {
    let new = pins.matrix();
    let old = MATRIX.lock_mutex();
    for (ind, (old_row, new_row)) in old.value.rows.iter().zip(new.rows.iter()).enumerate() {
        if old_row != new_row {
            //let l = DEBUG.lock_debugger();
            //let _ = l.value.write_fmt(format_args!("Row {ind} {} -> {}", old_row.on, new_row.on));
        }
    }
    *old.value = new;

}