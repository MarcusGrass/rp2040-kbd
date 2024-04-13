pub(crate) mod message_serializer;

use crate::keyboard::debounce::PinDebouncer;
use crate::keyboard::right::message_serializer::MessageSerializer;
use crate::keyboard::ButtonPin;
use crate::runtime::shared::cores_right::{push_reboot_and_halt, Producer};
#[cfg(feature = "serial")]
use core::fmt::Write;
use embedded_hal::digital::InputPin;
use rp2040_hal::gpio::bank0::{
    Gpio20, Gpio21, Gpio22, Gpio23, Gpio26, Gpio27, Gpio29, Gpio4, Gpio5, Gpio6, Gpio7, Gpio8,
    Gpio9,
};
use rp2040_hal::gpio::{FunctionSio, Pin, PinState, PullUp, SioInput};
use rp2040_hal::Timer;
use rp2040_kbd_lib::matrix::{ColIndex, MatrixIndex, MatrixUpdate, RowIndex};

const ROW0: u32 = 1 << 29;
const ROW1: u32 = 1 << 4;
const ROW2: u32 = 1 << 20;
const ROW3: u32 = 1 << 23;
const ROW4: u32 = 1 << 21;
const ROW_MASK: u32 = ROW0 | ROW1 | ROW2 | ROW3 | ROW4;

struct PinStructState {
    pressed: bool,
    debounce: PinDebouncer,
}

impl PinStructState {
    const fn new() -> Self {
        Self {
            pressed: false,
            debounce: PinDebouncer::new(),
        }
    }
}

macro_rules! pins_container {
    ($($row: tt, $col: tt),*,) => {
        paste::paste! {
            #[allow(clippy::struct_field_names)]
            struct RightPinsContainer {
                $(
                    [<row _ $row _ col _ $col _ state>] : PinStructState,
                )*
            }

            impl RightPinsContainer {
                const fn new() -> Self {
                    Self {
                        $(
                            [<row _ $row _ col _ $col _ state>] : PinStructState::new(),
                        )*
                    }
                }
            }
        }

    };
}

pins_container!(
    0, 0, 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 1, 0, 1, 1, 1, 2, 1, 3, 1, 4, 1, 5, 2, 0, 2, 1, 2, 2, 2, 3,
    2, 4, 2, 5, 3, 0, 3, 1, 3, 2, 3, 3, 3, 4, 3, 5, // 4, 0, Does not exist
    4, 1, 4, 2, 4, 3, 4, 4,
    // 4, 5, Has a rotary encoder on it
);

macro_rules! impl_check_rows_by_column {
    ($($structure: expr, $row: tt,)*, $col: tt) => {
        paste::paste! {
            #[inline]
            pub fn [<read_col _ $col _pins>](right_buttons: &mut RightButtons, serializer: &mut MessageSerializer, timer: Timer, changes: &mut u16, producer: &Producer) {

                // Safety: Make sure this is properly initialized and restored
                // at the end of this function, makes a noticeable difference in performance
                let col = unsafe {right_buttons.cols.$col.0.take().unwrap_unchecked()};
                let col = col.into_push_pull_output_in_state(PinState::Low);
                // Just pulling chibios defaults of 0.25 micros, could probably be 0
                crate::timer::wait_nanos(timer, 250);
                let bank = rp2040_hal::Sio::read_bank0();
                $(
                    {
                        const PRESSED: MatrixUpdate = MatrixUpdate::from_key_update(MatrixIndex::from_row_col(RowIndex::from_value($row), ColIndex::from_value($col)), true);
                        const RELEASED: MatrixUpdate = MatrixUpdate::from_key_update(MatrixIndex::from_row_col(RowIndex::from_value($row), ColIndex::from_value($col)), false);
                        let pressed = bank & [<ROW $row>] == 0;
                        #[cfg(feature = "serial")]
                        {
                            if right_buttons.pin_states.[< $structure:snake >].pressed != pressed {
                                let _ = crate::runtime::shared::usb::acquire_usb().write_fmt(format_args!(
                                    "M{}, R{}, C{} -> {} {:?}\r\n",
                                    MatrixIndex::from_row_col(RowIndex::from_value($row), ColIndex::from_value($col)).byte(),
                                    $row,
                                    $col,
                                    u8::from(pressed),
                                    right_buttons.pin_states.[< $structure:snake >].debounce.diff_last(timer.get_counter()),
                                ));
                            }

                        }
                        if right_buttons.pin_states.[< $structure:snake >].pressed != pressed && right_buttons.pin_states.[< $structure:snake >].debounce.try_submit(timer.get_counter(), pressed) {

                            serializer.serialize_matrix_state(if pressed {PRESSED} else {RELEASED});
                            right_buttons.pin_states.[< $structure:snake >].pressed = pressed;
                            *changes += 1;
                            if $row == 4 && $col == 2 {
                                push_reboot_and_halt(producer);
                            }
                        }
                    }
                )*
                right_buttons.cols.$col.0 = Some(col.into_pull_up_input());
                while rp2040_hal::Sio::read_bank0() & ROW_MASK != ROW_MASK {}
            }
        }
    };
}

impl_check_rows_by_column!(
    row_0_col_0_state, 0,
    row_1_col_0_state, 1,
    row_2_col_0_state, 2,
    row_3_col_0_state, 3,
    ,0
);

impl_check_rows_by_column!(
    row_0_col_1_state, 0,
    row_1_col_1_state, 1,
    row_2_col_1_state, 2,
    row_3_col_1_state, 3,
    row_4_col_1_state, 4,
    ,1
);

impl_check_rows_by_column!(
    row_0_col_2_state, 0,
    row_1_col_2_state, 1,
    row_2_col_2_state, 2,
    row_3_col_2_state, 3,
    row_4_col_2_state, 4,
    ,2
);

impl_check_rows_by_column!(
    row_0_col_3_state, 0,
    row_1_col_3_state, 1,
    row_2_col_3_state, 2,
    row_3_col_3_state, 3,
    row_4_col_3_state, 4,
    ,3
);

impl_check_rows_by_column!(
    row_0_col_4_state, 0,
    row_1_col_4_state, 1,
    row_2_col_4_state, 2,
    row_3_col_4_state, 3,
    row_4_col_4_state, 4,
    ,4
);

impl_check_rows_by_column!(
    row_0_col_5_state, 0,
    row_1_col_5_state, 1,
    row_2_col_5_state, 2,
    row_3_col_5_state, 3,
    ,5
);

pub struct RightButtons {
    pin_states: RightPinsContainer,
    _rows: (
        ButtonPin<Gpio29>,
        ButtonPin<Gpio4>,
        ButtonPin<Gpio20>,
        ButtonPin<Gpio23>,
        ButtonPin<Gpio21>,
    ),
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
        // Want this supplied owned and in the correct state
        #[allow(clippy::used_underscore_binding)] _rows: (
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
            pin_states: RightPinsContainer::new(),
            _rows,
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
    pub fn scan_matrix(
        &mut self,
        serializer: &mut MessageSerializer,
        timer: Timer,
        producer: &Producer,
    ) -> u16 {
        let mut changes = 0;
        read_col_0_pins(self, serializer, timer, &mut changes, producer);
        read_col_1_pins(self, serializer, timer, &mut changes, producer);
        read_col_2_pins(self, serializer, timer, &mut changes, producer);
        read_col_3_pins(self, serializer, timer, &mut changes, producer);
        read_col_4_pins(self, serializer, timer, &mut changes, producer);
        read_col_5_pins(self, serializer, timer, &mut changes, producer);
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
