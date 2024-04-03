pub(crate) mod message_serializer;

use crate::keyboard::right::message_serializer::MessageSerializer;
use crate::keyboard::usb_serial::UsbSerial;
use crate::keyboard::{
    matrix_ind, ButtonPin, ButtonState, ButtonStateChange, MatrixState, MatrixUpdate, RowPin,
    INITIAL_STATE, NUM_COLS, NUM_ROWS,
};
use core::fmt::Write;
use embedded_hal::digital::v2::{InputPin, OutputPin, PinState};
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_hal::gpio::bank0::{
    Gpio20, Gpio21, Gpio22, Gpio23, Gpio26, Gpio27, Gpio29, Gpio4, Gpio5, Gpio6, Gpio7, Gpio8,
    Gpio9,
};
use rp2040_hal::gpio::{FunctionSio, OutputSlewRate, Pin, PinId, PullDown, PullUp, SioInput};
use rp2040_hal::timer::Instant;
use rp2040_hal::Timer;

#[derive(Copy, Clone, Debug)]
pub struct JitterRegulator {
    last_touch: Option<Instant>,
    quarantined: Option<bool>,
}

impl JitterRegulator {
    const fn new() -> Self {
        Self {
            last_touch: None,
            quarantined: None,
        }
    }
    fn try_submit(&mut self, now: Instant, state: bool) -> bool {
        let Some(earlier) = self.last_touch else {
            self.last_touch = Some(now);
            return true;
        };
        self.last_touch = Some(now);
        let Some(diff) = now.checked_duration_since(earlier) else {
            return true;
        };
        // Effectively, maximum presses per single key becomes is 1 per 10 millis, that's a 1200 WPM with a single finger
        if diff.to_micros() < 10_000 {
            self.quarantined = Some(state);
            return false;
        }

        self.quarantined.take();
        true
    }

    fn try_release_quarantined(&mut self, now: Instant) -> Option<bool> {
        let quarantined = self.quarantined?;
        let last = self.last_touch?;
        let diff = now.checked_duration_since(last)?;
        if diff.to_millis() < 10_000 {
            return None;
        }
        self.quarantined.take()
    }
}

pub struct RightButtons {
    pub(crate) matrix: MatrixState,
    pub(crate) changes: [JitterRegulator; NUM_ROWS * NUM_COLS],
    rows: [RowPin; 5],
    cols: (
        Option<ButtonPin<Gpio22>>,
        Option<ButtonPin<Gpio5>>,
        Option<ButtonPin<Gpio6>>,
        Option<ButtonPin<Gpio7>>,
        Option<ButtonPin<Gpio8>>,
        Option<ButtonPin<Gpio9>>,
    ),
    encoder: RotaryEncoder,
}

impl RightButtons {
    pub fn new(
        mut rows: (
            ButtonPin<Gpio29>,
            ButtonPin<Gpio4>,
            ButtonPin<Gpio20>,
            ButtonPin<Gpio23>,
            ButtonPin<Gpio21>,
        ),
        mut cols: (
            ButtonPin<Gpio22>,
            ButtonPin<Gpio5>,
            ButtonPin<Gpio6>,
            ButtonPin<Gpio7>,
            ButtonPin<Gpio8>,
            ButtonPin<Gpio9>,
        ),
        rotary_encoder: RotaryEncoder,
    ) -> Self {
        Self {
            matrix: INITIAL_STATE,
            changes: [JitterRegulator::new(); NUM_COLS * NUM_ROWS],
            rows: [
                rows.0.into_dyn_pin(),
                rows.1.into_dyn_pin(),
                rows.2.into_dyn_pin(),
                rows.3.into_dyn_pin(),
                rows.4.into_dyn_pin(),
            ],
            cols: (
                Some(
                    cols.0
                        .into_push_pull_output_in_state(PinState::High)
                        .into_function(),
                ),
                Some(
                    cols.1
                        .into_push_pull_output_in_state(PinState::High)
                        .into_function(),
                ),
                Some(
                    cols.2
                        .into_push_pull_output_in_state(PinState::High)
                        .into_function(),
                ),
                Some(
                    cols.3
                        .into_push_pull_output_in_state(PinState::High)
                        .into_function(),
                ),
                Some(
                    cols.4
                        .into_push_pull_output_in_state(PinState::High)
                        .into_function(),
                ),
                Some(
                    cols.5
                        .into_push_pull_output_in_state(PinState::High)
                        .into_function(),
                ),
            ),
            encoder: rotary_encoder,
        }
    }

    pub fn scan_matrix(&mut self, serializer: &mut MessageSerializer, timer: Timer) -> bool {
        let col0_change = check_col::<0, _>(
            &mut self.cols.0,
            &mut self.rows,
            &mut self.matrix,
            &mut self.changes,
            serializer,
            timer,
        );
        let col1_change = check_col::<1, _>(
            &mut self.cols.1,
            &mut self.rows,
            &mut self.matrix,
            &mut self.changes,
            serializer,
            timer,
        );
        let col2_change = check_col::<2, _>(
            &mut self.cols.2,
            &mut self.rows,
            &mut self.matrix,
            &mut self.changes,
            serializer,
            timer,
        );
        let col3_change = check_col::<3, _>(
            &mut self.cols.3,
            &mut self.rows,
            &mut self.matrix,
            &mut self.changes,
            serializer,
            timer,
        );
        let col4_change = check_col::<4, _>(
            &mut self.cols.4,
            &mut self.rows,
            &mut self.matrix,
            &mut self.changes,
            serializer,
            timer,
        );
        let col5_change = check_col::<5, _>(
            &mut self.cols.5,
            &mut self.rows,
            &mut self.matrix,
            &mut self.changes,
            serializer,
            timer,
        );
        let now = timer.get_counter();
        let mut changed = false;
        for (matrix_ind, jitter) in self.changes.iter_mut().enumerate() {
            let Some(state) = jitter.try_release_quarantined(now) else {
                continue;
            };
            if state == self.matrix[matrix_ind] {
                continue;
            }
            #[cfg(feature = "serial")]
            {
                let _ = crate::runtime::shared::usb::acquire_usb().write_fmt(format_args!(
                    "Release Quarantined M{}-> {}\r\n",
                    matrix_ind, state as u8
                ));
            }
            serializer.serialize_matrix_state(&crate::keyboard::MatrixUpdate::new_keypress(
                matrix_ind as u8,
                state,
            ));
            changed = true;
        }
        col0_change
            || col1_change
            || col2_change
            || col3_change
            || col4_change
            || col5_change
            || changed
    }

    #[inline]
    pub fn scan_encoder(&mut self, serializer: &mut MessageSerializer) -> bool {
        if let Some(dir) = self.encoder.scan_debounced() {
            #[cfg(feature = "serial")]
            {
                let _ = crate::runtime::shared::usb::acquire_usb().write_fmt(format_args!(
                    "Encoder clockwise: Pos: {:?} clockwise={:?}\r\n",
                    self.encoder.last_position, dir
                ));
            }
            serializer.serialize_matrix_state(&MatrixUpdate::new_encoder_rotation(dir));
            true
        } else {
            false
        }
    }
}

fn check_col<
    const N: usize,
    Id: PinId
        + rp2040_hal::gpio::ValidFunction<FunctionSio<SioInput>>
        + rp2040_hal::gpio::ValidFunction<FunctionSio<rp2040_hal::gpio::SioOutput>>,
>(
    input: &mut Option<ButtonPin<Id>>,
    rows: &mut [RowPin],
    matrix: &mut MatrixState,
    jitters: &mut [JitterRegulator; NUM_ROWS * NUM_COLS],
    serializer: &mut MessageSerializer,
    timer: Timer,
) -> bool {
    let mut col = input.take().unwrap();
    let mut col = col.into_push_pull_output_in_state(PinState::Low);
    let mut cd = timer.count_down();
    embedded_hal::timer::CountDown::start(&mut cd, rp2040_hal::fugit::MicrosDurationU64::micros(1));
    nb::block!(embedded_hal::timer::CountDown::wait(&mut cd));
    let mut changed = false;
    for (row_ind, row_pin) in rows.iter().enumerate() {
        let ind = matrix_ind(row_ind, N);
        let state = loop {
            if let Ok(val) = row_pin.is_low() {
                break val;
            }
        };
        if state != matrix[ind] {
            if !jitters[ind].try_submit(timer.get_counter(), state) {
                #[cfg(feature = "serial")]
                {
                    let _ = crate::runtime::shared::usb::acquire_usb().write_fmt(format_args!(
                        "Quarantine: M{}, R{}, C{} -> {}\r\n",
                        ind, row_ind, N, state as u8
                    ));
                }
                continue;
            }

            #[cfg(feature = "serial")]
            {
                let _ = crate::runtime::shared::usb::acquire_usb().write_fmt(format_args!(
                    "M{}, R{}, C{} -> {}\r\n",
                    ind, row_ind, N, state as u8
                ));
            }
            serializer.serialize_matrix_state(&crate::keyboard::MatrixUpdate::new_keypress(
                ind as u8, state,
            ));
            // Todo: Make this less esoteric
            if N == 2 && row_ind == 4 {
                rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
            }
            changed = true;
            matrix.set(ind, state);
        }
    }
    *input = Some(col.into_pull_up_input());
    // Wait for all rows to settle
    for row in rows {
        while matches!(row.is_low(), Ok(true)) {}
    }
    changed
}

#[derive(Copy, Clone, Debug)]
enum RotaryPosition {
    North,
    East,
    South,
    West,
}

impl RotaryPosition {
    fn from_state(a: bool, b: bool) -> Self {
        match (a, b) {
            (true, true) => RotaryPosition::South,
            (false, true) => RotaryPosition::West,
            (false, false) => RotaryPosition::North,
            (true, false) => RotaryPosition::East,
        }
    }
}

pub struct RotaryEncoder {
    pin_a: Pin<Gpio26, FunctionSio<SioInput>, PullUp>,
    pin_b: Pin<Gpio27, FunctionSio<SioInput>, PullUp>,
    last_position: Option<RotaryPosition>,
    last_clockwise: Option<bool>,
    cached: Option<bool>,
}

impl RotaryEncoder {
    pub fn new(
        pin_a: Pin<Gpio26, FunctionSio<SioInput>, PullUp>,
        pin_b: Pin<Gpio27, FunctionSio<SioInput>, PullUp>,
    ) -> Self {
        Self {
            pin_a,
            pin_b,
            last_position: None,
            last_clockwise: None,
            cached: None,
        }
    }

    #[inline]
    fn read_position(&self) -> RotaryPosition {
        let new_pin_a_state = matches!(self.pin_a.is_high(), Ok(true));
        let new_pin_b_state = matches!(self.pin_b.is_high(), Ok(true));
        RotaryPosition::from_state(new_pin_a_state, new_pin_b_state)
    }

    // Dirty, but works for debouncing the encoder
    #[inline]
    pub fn scan_debounced(&mut self) -> Option<bool> {
        let current = self.read_position();
        let Some(old) = self.last_position else {
            self.last_position = Some(current);
            return None;
        };
        let dir = match (old, current) {
            (RotaryPosition::North, RotaryPosition::East)
            | (RotaryPosition::East, RotaryPosition::South)
            | (RotaryPosition::South, RotaryPosition::West)
            | (RotaryPosition::West, RotaryPosition::North) => true,
            (RotaryPosition::North, RotaryPosition::West)
            | (RotaryPosition::West, RotaryPosition::South)
            | (RotaryPosition::South, RotaryPosition::East)
            | (RotaryPosition::East, RotaryPosition::North) => false,
            (_, _) => {
                self.last_position = Some(current);
                return None;
            }
        };
        self.last_position = Some(current);
        let Some(last) = self.last_clockwise else {
            self.last_clockwise = Some(dir);
            return None;
        };
        self.last_clockwise = Some(dir);
        if last == dir {
            if let Some(prev) = self.cached {
                if prev == last {
                    return self.cached.take();
                }
            }
            self.cached = Some(dir);
        }
        None
    }
}
