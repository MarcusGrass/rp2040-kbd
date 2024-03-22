use embedded_hal::digital::v2::{InputPin, PinState};
use rp2040_hal::gpio::bank0::{
    Gpio20, Gpio21, Gpio22, Gpio23, Gpio26, Gpio27, Gpio29, Gpio6, Gpio7, Gpio8, Gpio9,
};
use rp2040_hal::gpio::{DynPinId, FunctionSio, Pin, PinId, PullUp, SioInput};

type RowPin = Pin<DynPinId, FunctionSio<SioInput>, PullUp>;
type ButtonPin<Id> = Pin<Id, FunctionSio<SioInput>, PullUp>;

pub const NUM_COLS: usize = 6;
pub const NUM_ROWS: usize = 5;

#[derive(Debug, Copy, Clone)]
pub enum KeyboardRow {
    One,
    Two,
    Three,
    Four,
    Five,
}

#[derive(Debug, Copy, Clone)]
pub enum KeyboardCol {
    One,
    Two,
    Three,
    Four,
    Five,
    Size,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ButtonState {
    Depressed = 0,
    Pressed = 1,
}

#[derive(Debug, Copy, Clone)]
pub struct ButtonStateChange {
    row: u8,
    col: u8,
    new_state: ButtonState,
}

pub struct Left {
    prev_matrix: MatrixState,
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

pub type MatrixState = [[ButtonState; NUM_COLS]; NUM_ROWS];

const INITIAL_STATE: MatrixState = [[ButtonState::Depressed; NUM_COLS]; NUM_ROWS];

macro_rules! check_col {
    ($slf: expr, $pt: ident, $m_state: expr, $vec: expr) => {
        {
            let mut col = $slf.$pt.take().unwrap();
            let col0 = col0.into_push_pull_output_in_state(PinState::Low);
            for (ind, row) in $slf.rows.iter().enumerate() {
                let state = if matches!(row.is_low(), Ok(true)) {
                    ButtonState::Pressed
                } else {
                    ButtonState::Depressed
                };
                if state != $slf.prev_matrix[ind][0] {
                    let _ = $vec.push(ButtonStateChange {
                        row: ind as u8,
                        col: 0,
                        new_state: state,
                    });
                }
                next_state[ind][0] = state;
            }
        }

    };
}
impl Left {
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
            prev_matrix: INITIAL_STATE,
            rows: [rows.0.into_dyn_pin(), rows.1.into_dyn_pin(), rows.2.into_dyn_pin(), rows.3.into_dyn_pin(), rows.4.into_dyn_pin()],
            cols,
        }
    }

    pub fn scan_matrix(&mut self) -> heapless::Vec<ButtonStateChange, 16> {
        let mut col0 = self.cols.0.take().unwrap();
        let mut next_state = INITIAL_STATE;
        let mut changes = heapless::Vec::new();
        let col0 = col0.into_push_pull_output_in_state(PinState::Low);
        for (ind, row) in self.rows.iter().enumerate() {
            let state = if matches!(row.is_low(), Ok(true)) {
                ButtonState::Pressed
            } else {
                ButtonState::Depressed
            };
            if state != self.prev_matrix[ind][0] {
                let _ = changes.push(ButtonStateChange {
                    row: ind as u8,
                    col: 0,
                    new_state: state,
                });
            }
            next_state[ind][0] = state;
        }
        changes
    }

}
