pub(crate) mod message_serializer;

use crate::keyboard::debounce::PinDebouncer;
use crate::keyboard::right::message_serializer::MessageSerializer;
use crate::keyboard::{ButtonPin, RowPin, INITIAL_STATE};
use crate::runtime::shared::cores_right::push_reboot_and_halt;
#[cfg(feature = "serial")]
use core::fmt::Write;
use embedded_hal::digital::InputPin;
use rp2040_hal::gpio::bank0::{
    Gpio20, Gpio21, Gpio22, Gpio23, Gpio26, Gpio27, Gpio29, Gpio4, Gpio5, Gpio6, Gpio7, Gpio8,
    Gpio9,
};
use rp2040_hal::gpio::{FunctionSio, Pin, PinId, PinState, PullUp, SioInput};
use rp2040_hal::Timer;
use rp2040_kbd_lib::matrix::{
    ColIndex, MatrixIndex, MatrixState, MatrixUpdate, RowIndex, NUM_COLS, NUM_ROWS,
};

#[derive(Copy, Clone)]
pub struct PinDebouncers([PinDebouncer; (NUM_ROWS * NUM_COLS) as usize]);

impl PinDebouncers {
    pub const fn new() -> Self {
        Self([PinDebouncer::new(); (NUM_ROWS * NUM_COLS) as usize])
    }

    #[inline]
    #[cfg(feature = "serial")]
    pub fn get(&self, matrix_index: MatrixIndex) -> &PinDebouncer {
        unsafe { self.0.get_unchecked(matrix_index.index()) }
    }

    #[inline]
    pub fn get_mut(&mut self, matrix_index: MatrixIndex) -> &mut PinDebouncer {
        unsafe { self.0.get_unchecked_mut(matrix_index.index()) }
    }
}

pub struct RightButtons {
    pub(crate) matrix: MatrixState,
    pub(crate) changes: PinDebouncers,
    rows: [(RowPin, RowIndex); 5],
    cols: (
        (Option<ButtonPin<Gpio22>>, ColIndex),
        (Option<ButtonPin<Gpio5>>, ColIndex),
        (Option<ButtonPin<Gpio6>>, ColIndex),
        (Option<ButtonPin<Gpio7>>, ColIndex),
        (Option<ButtonPin<Gpio8>>, ColIndex),
        (Option<ButtonPin<Gpio9>>, ColIndex),
    ),
    encoder: RotaryEncoder,
}

impl RightButtons {
    pub fn new(
        rows: (
            ButtonPin<Gpio29>,
            ButtonPin<Gpio4>,
            ButtonPin<Gpio20>,
            ButtonPin<Gpio23>,
            ButtonPin<Gpio21>,
        ),
        cols: (
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
            changes: PinDebouncers::new(),
            rows: [
                (rows.0.into_dyn_pin(), RowIndex::from_value(0)),
                (rows.1.into_dyn_pin(), RowIndex::from_value(1)),
                (rows.2.into_dyn_pin(), RowIndex::from_value(2)),
                (rows.3.into_dyn_pin(), RowIndex::from_value(3)),
                (rows.4.into_dyn_pin(), RowIndex::from_value(4)),
            ],
            cols: (
                (
                    Some(
                        cols.0
                            .into_push_pull_output_in_state(PinState::High)
                            .into_function(),
                    ),
                    ColIndex::from_value(0),
                ),
                (
                    Some(
                        cols.1
                            .into_push_pull_output_in_state(PinState::High)
                            .into_function(),
                    ),
                    ColIndex::from_value(1),
                ),
                (
                    Some(
                        cols.2
                            .into_push_pull_output_in_state(PinState::High)
                            .into_function(),
                    ),
                    ColIndex::from_value(2),
                ),
                (
                    Some(
                        cols.3
                            .into_push_pull_output_in_state(PinState::High)
                            .into_function(),
                    ),
                    ColIndex::from_value(3),
                ),
                (
                    Some(
                        cols.4
                            .into_push_pull_output_in_state(PinState::High)
                            .into_function(),
                    ),
                    ColIndex::from_value(4),
                ),
                (
                    Some(
                        cols.5
                            .into_push_pull_output_in_state(PinState::High)
                            .into_function(),
                    ),
                    ColIndex::from_value(5),
                ),
            ),
            encoder: rotary_encoder,
        }
    }

    #[allow(clippy::cast_lossless, clippy::cast_possible_truncation)]
    pub fn scan_matrix(&mut self, serializer: &mut MessageSerializer, timer: Timer) -> u16 {
        let mut changes = 0;
        changes += check_col::<0, _>(
            &mut self.cols.0,
            &mut self.rows,
            &mut self.matrix,
            &mut self.changes,
            serializer,
            timer,
        );
        changes += check_col::<1, _>(
            &mut self.cols.1,
            &mut self.rows,
            &mut self.matrix,
            &mut self.changes,
            serializer,
            timer,
        );
        changes += check_col::<2, _>(
            &mut self.cols.2,
            &mut self.rows,
            &mut self.matrix,
            &mut self.changes,
            serializer,
            timer,
        );
        changes += check_col::<3, _>(
            &mut self.cols.3,
            &mut self.rows,
            &mut self.matrix,
            &mut self.changes,
            serializer,
            timer,
        );
        changes += check_col::<4, _>(
            &mut self.cols.4,
            &mut self.rows,
            &mut self.matrix,
            &mut self.changes,
            serializer,
            timer,
        );
        changes += check_col::<5, _>(
            &mut self.cols.5,
            &mut self.rows,
            &mut self.matrix,
            &mut self.changes,
            serializer,
            timer,
        );
        changes
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
            serializer.serialize_matrix_state(MatrixUpdate::from_rotary_change(dir));
            true
        } else {
            false
        }
    }
}

#[allow(clippy::cast_lossless, clippy::cast_possible_truncation)]
fn check_col<
    const N: usize,
    Id: PinId
        + rp2040_hal::gpio::ValidFunction<FunctionSio<SioInput>>
        + rp2040_hal::gpio::ValidFunction<FunctionSio<rp2040_hal::gpio::SioOutput>>,
>(
    input: &mut (Option<ButtonPin<Id>>, ColIndex),
    rows: &mut [(RowPin, RowIndex)],
    matrix: &mut MatrixState,
    debouncers: &mut PinDebouncers,
    serializer: &mut MessageSerializer,
    timer: Timer,
) -> u16 {
    // Safety, ensure this is properly initalized in constructor,
    // and restore at the end of this function,
    // makes a noticeable difference vs unwrap
    let col = unsafe { input.0.take().unwrap_unchecked() };
    let col = col.into_push_pull_output_in_state(rp2040_hal::gpio::PinState::Low);
    crate::timer::wait_nanos(timer, 250);
    let mut changed = 0;
    for (row_pin, row_ind) in rows.iter_mut() {
        // This should be known at comptime
        let ind = MatrixIndex::from_row_col(*row_ind, input.1);
        let state = loop {
            if let Ok(val) = row_pin.is_low() {
                break val;
            }
        };
        if state != matrix.get(ind) {
            #[cfg(feature = "serial")]
            {
                let _ = crate::runtime::shared::usb::acquire_usb().write_fmt(format_args!(
                    "M{}, R{}, C{} -> {} {:?}\r\n",
                    ind.byte(),
                    row_ind.0,
                    N,
                    u8::from(state),
                    debouncers.get(ind).diff_last(timer.get_counter()),
                ));
            }
            if !debouncers
                .get_mut(ind)
                .try_submit(timer.get_counter(), state)
            {
                #[cfg(feature = "serial")]
                {
                    let _ = crate::runtime::shared::usb::acquire_usb().write_fmt(format_args!(
                        "Quarantine: M{}, R{}, C{} -> {}\r\n",
                        ind.byte(),
                        row_ind.0,
                        N,
                        u8::from(state),
                    ));
                }
                continue;
            }
            serializer.serialize_matrix_state(MatrixUpdate::from_key_update(ind, state));
            // Todo: Make this less esoteric
            if N == 2 && row_ind.0 == 4 {
                push_reboot_and_halt();
            }
            changed += 1;
            matrix.set(ind, state);
        }
    }
    input.0 = Some(col.into_pull_up_input());
    // Wait for all rows to settle
    for row in rows {
        while matches!(row.0.is_low(), Ok(true)) {}
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
    dt_pin: Pin<Gpio26, FunctionSio<SioInput>, PullUp>,
    clk_pin: Pin<Gpio27, FunctionSio<SioInput>, PullUp>,
    last_position: Option<RotaryPosition>,
    last_clockwise: Option<bool>,
    cached: Option<bool>,
}

impl RotaryEncoder {
    pub fn new(
        dt_pin: Pin<Gpio26, FunctionSio<SioInput>, PullUp>,
        clk_pin: Pin<Gpio27, FunctionSio<SioInput>, PullUp>,
    ) -> Self {
        Self {
            dt_pin,
            clk_pin,
            last_position: None,
            last_clockwise: None,
            cached: None,
        }
    }

    #[inline]
    fn read_position(&mut self) -> RotaryPosition {
        RotaryPosition::from_state(
            matches!(self.dt_pin.is_high(), Ok(true)),
            matches!(self.clk_pin.is_high(), Ok(true)),
        )
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
