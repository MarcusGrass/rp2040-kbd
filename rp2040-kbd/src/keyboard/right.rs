use rp2040_hal::gpio::bank0::{Gpio20, Gpio21, Gpio22, Gpio23, Gpio26, Gpio29, Gpio4, Gpio9};
use crate::check_col;
use crate::keyboard::{ButtonPin, ButtonState, ButtonStateChange, INITIAL_STATE, MatrixState, RowPin};
use embedded_hal::digital::v2::{InputPin, OutputPin, PinState};

pub struct RightButtons {
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
        let mut next_state = INITIAL_STATE;
        let mut changes = heapless::Vec::new();
        check_col!(self, 0, next_state, changes);
        check_col!(self, 1, next_state, changes);
        check_col!(self, 2, next_state, changes);
        check_col!(self, 3, next_state, changes);
        check_col!(self, 4, next_state, changes);
        // Todo: Row 4 gets weird, may be because it has fewer buttons, may be wrongly mapped
        check_col!(self, 5, next_state, changes);
        self.prev_matrix = next_state;
        changes
    }

}