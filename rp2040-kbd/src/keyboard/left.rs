pub(crate) mod message_receiver;

use core::fmt::Write;
use rp2040_hal::gpio::bank0::{Gpio20, Gpio21, Gpio22, Gpio23, Gpio26, Gpio27, Gpio29, Gpio6, Gpio7, Gpio8, Gpio9};
use crate::keyboard::{ButtonPin, ButtonState, ButtonStateChange, INITIAL_STATE, MatrixState, NUM_COLS, NUM_ROWS, RowPin, matrix_ind};
use embedded_hal::digital::v2::{InputPin, OutputPin, PinState};
use usbd_hid::descriptor::KeyboardReport;
use crate::keyboard::usb_serial::UsbSerial;

pub struct LeftButtons {
    matrix: MatrixState,
    rows: [
        RowPin; 5
    ],
    cols: (
        Option<ButtonPin<Gpio9>>,
        Option<ButtonPin<Gpio26>>,
        Option<ButtonPin<Gpio22>>,
        Option<ButtonPin<Gpio20>>,
        Option<ButtonPin<Gpio23>>,
        Option<ButtonPin<Gpio21>>,
    ),
}

impl LeftButtons {
    pub fn new(
        rows: (
            ButtonPin<Gpio29>,
            ButtonPin<Gpio27>,
            ButtonPin<Gpio6>,
            ButtonPin<Gpio7>,
            ButtonPin<Gpio8>,
        ),
        cols: (
            Option<ButtonPin<Gpio9>>,
            Option<ButtonPin<Gpio26>>,
            Option<ButtonPin<Gpio22>>,
            Option<ButtonPin<Gpio20>>,
            Option<ButtonPin<Gpio23>>,
            Option<ButtonPin<Gpio21>>,
        ),
    ) -> Self {
        Self {
            matrix: INITIAL_STATE,
            rows: [rows.0.into_dyn_pin(), rows.1.into_dyn_pin(), rows.2.into_dyn_pin(), rows.3.into_dyn_pin(), rows.4.into_dyn_pin()],
            cols,
        }
    }

    pub fn scan_matrix(&mut self) -> heapless::Vec<ButtonStateChange, 16> {
        let mut next_state = INITIAL_STATE;
        let mut changes = heapless::Vec::new();
        crate::check_col_no_store!(self, 0, next_state);
        crate::check_col_no_store!(self, 1, next_state);
        crate::check_col_no_store!(self, 2, next_state);
        crate::check_col_no_store!(self, 3, next_state);
        crate::check_col_no_store!(self, 4, next_state);
        // Todo: Row 4 gets weird, may be because it has fewer buttons, may be wrongly mapped
        crate::check_col_no_store!(self, 5, next_state);
        self.matrix = next_state;
        changes
    }
}

#[derive(Debug)]
pub struct KeyboardState<const N: usize> {
    left: MatrixState,
    right: MatrixState,
    hid_state: KeyboardReport,

}

impl<const N: usize> KeyboardState<N> {
    pub const fn empty() -> Self {
        Self {
            left: INITIAL_STATE,
            right: INITIAL_STATE,
            hid_state: KeyboardReport {
                modifier: 0,
                reserved: 0,
                leds: 0,
                keycodes: [0; 6],
            },
        }
    }

    pub fn update_left(&mut self, new: &MatrixState, usb_serial: &mut UsbSerial) -> bool {
        Self::update(&mut self.right, new, usb_serial, true)
    }

    pub fn update_right(&mut self, new: &MatrixState, usb_serial: &mut UsbSerial) -> bool {
        Self::update(&mut self.right, new, usb_serial, false)
    }

    #[inline]
    fn update(side: &mut MatrixState, new: &MatrixState, usb_serial: &mut UsbSerial, left: bool) -> bool {
        let mut any = false;
        for row_ind in 0..NUM_ROWS {
            for col_ind in 0..NUM_COLS {
                let ind = matrix_ind(row_ind, col_ind);
                let old_val = side[ind];
                let new_val = new[ind];
                if old_val != new_val {
                    if left {
                        let _ = usb_serial.write_fmt(format_args!("L: R{},C{} -> {}\r\n", row_ind, col_ind, new_val as u8));
                    } else {
                        let _ = usb_serial.write_fmt(format_args!("R: R{},C{} -> {}\r\n", row_ind, col_ind, new_val as u8));
                    }
                    side.set(ind, new_val);
                    any = true;
                }
            }
        }
        any
    }

}