use core::hash::BuildHasherDefault;
use heapless::IndexMap;
use rp2040_hal::rom_data::reset_to_usb_boot;
use crate::hid::keycodes::{KeyCode, Modifier};
use crate::keyboard::{matrix_ind, MatrixState, NUM_COLS, NUM_ROWS};
use usbd_hid::descriptor::KeyboardReport;
use paste::paste;
use crate::keyboard::left::LeftButtons;
use rp2040_hal::gpio::PinState;
use embedded_hal::digital::v2::{InputPin, OutputPin};


#[derive(Debug, Copy, Clone)]
pub enum KeymapLayer {
    DvorakAnsi,
    DvorakSe,
}

#[derive(Debug, Copy, Clone)]
pub struct LayerResult {
    pub next_layer: Option<KeymapLayer>,
    pub report: KeyboardReport,
}

impl KeymapLayer {
    pub fn report(self, left: &MatrixState, right: &MatrixState) -> LayerResult {
        LayerResult {
            next_layer: None,
            report: KeyboardReport {
                modifier: 0,
                reserved: 0,
                leds: 0,
                keycodes: [0u8; 6],
            },
        }
    }
}

// Make sure that the index calculation is const
macro_rules! at_ind {
    ($side: expr, $row: expr, $col: expr, $do: expr) => {{
        const IND: usize = matrix_ind($row, $col);
        if $side[IND] {
            $do
        }
    }};
}

macro_rules! at_ind_keycode {
    ($side: expr, $row: expr, $col: expr, $keycodes: expr, $code_ind: expr, $kc: expr) => {{
        at_ind!($side, $row, $col, {
            $keycodes[$code_ind] = $kc.0;
            $code_ind += 1;
        })
    }};
}

macro_rules! at_ind_mod {
    ($side: expr, $row: expr, $col: expr, $mods: expr, $mod_kc: expr) => {{
        at_ind!($side, $row, $col, {
            $mods |= $mod_kc.0;
        })
    }};
}

fn dvorak_se_to_report(left: &MatrixState, right: &MatrixState) -> LayerResult {
    let mut mods = 0u8;
    let mut code_ind = 0;
    let mut keycodes = [0u8; 6];
    at_ind_keycode!(left, 0, 0, keycodes, code_ind, KeyCode::KC_TAB);
    at_ind_keycode!(left, 0, 1, keycodes, code_ind, KeyCode::KC_QUOT);
    at_ind_keycode!(left, 0, 2, keycodes, code_ind, KeyCode::COMMA);
    at_ind_keycode!(left, 0, 3, keycodes, code_ind, KeyCode::DOT);
    at_ind_keycode!(left, 0, 4, keycodes, code_ind, KeyCode::P);
    at_ind_keycode!(left, 0, 5, keycodes, code_ind, KeyCode::Y);

    at_ind_keycode!(left, 1, 0, keycodes, code_ind, KeyCode::ESCAPE);
    at_ind_keycode!(left, 1, 1, keycodes, code_ind, KeyCode::A);
    at_ind_keycode!(left, 1, 2, keycodes, code_ind, KeyCode::O);
    at_ind_keycode!(left, 1, 3, keycodes, code_ind, KeyCode::E);
    at_ind_keycode!(left, 1, 4, keycodes, code_ind, KeyCode::U);
    at_ind_keycode!(left, 1, 5, keycodes, code_ind, KeyCode::I);

    at_ind_mod!(left, 2, 0, mods, Modifier::LEFT_SHIFT);
    at_ind_keycode!(left, 2, 1, keycodes, code_ind, KeyCode::SEMICOLON);
    at_ind_keycode!(left, 2, 2, keycodes, code_ind, KeyCode::Q);
    at_ind_keycode!(left, 2, 3, keycodes, code_ind, KeyCode::J);
    at_ind_keycode!(left, 2, 4, keycodes, code_ind, KeyCode::K);
    at_ind_keycode!(left, 2, 5, keycodes, code_ind, KeyCode::X);

    at_ind_mod!(left, 3, 0, mods, Modifier::LEFT_CONTROL);
    at_ind_keycode!(left, 3, 1, keycodes, code_ind, KeyCode::SEMICOLON);
    at_ind_keycode!(left, 3, 2, keycodes, code_ind, KeyCode::Q);
    at_ind_keycode!(left, 3, 3, keycodes, code_ind, KeyCode::J);
    at_ind_keycode!(left, 3, 4, keycodes, code_ind, KeyCode::K);
    at_ind_keycode!(left, 3, 5, keycodes, code_ind, KeyCode::KC_SPC);

    at_ind!(left, 4, 4, {
        reset_to_usb_boot(0, 0)
    });

    at_ind_keycode!(right, 0, 0, keycodes, code_ind, KeyCode::KC_DEL);
    at_ind_keycode!(right, 0, 1, keycodes, code_ind, KeyCode::KC_L);
    at_ind_keycode!(right, 0, 2, keycodes, code_ind, KeyCode::KC_R);
    at_ind_keycode!(right, 0, 3, keycodes, code_ind, KeyCode::KC_C);
    at_ind_keycode!(right, 0, 4, keycodes, code_ind, KeyCode::KC_G);
    at_ind_keycode!(right, 0, 5, keycodes, code_ind, KeyCode::KC_F);

    at_ind_keycode!(right, 1, 0, keycodes, code_ind, KeyCode::KC_RET);
    at_ind_keycode!(right, 1, 1, keycodes, code_ind, KeyCode::KC_S);
    at_ind_keycode!(right, 1, 2, keycodes, code_ind, KeyCode::KC_N);
    at_ind_keycode!(right, 1, 3, keycodes, code_ind, KeyCode::KC_T);
    at_ind_keycode!(right, 1, 4, keycodes, code_ind, KeyCode::KC_H);
    at_ind_keycode!(right, 1, 5, keycodes, code_ind, KeyCode::KC_D);

    at_ind_mod!(right, 2, 0, mods, Modifier::LEFT_SHIFT);
    at_ind_keycode!(right, 2, 1, keycodes, code_ind, KeyCode::KC_Z);
    at_ind_keycode!(right, 2, 2, keycodes, code_ind, KeyCode::KC_V);
    at_ind_keycode!(right, 2, 3, keycodes, code_ind, KeyCode::KC_W);
    at_ind_keycode!(right, 2, 4, keycodes, code_ind, KeyCode::KC_M);
    at_ind_keycode!(right, 2, 5, keycodes, code_ind, KeyCode::KC_B);

    // Unused, encoder is at 0
    at_ind_mod!(right, 3, 0, mods, Modifier::LEFT_CONTROL);
    /*
    at_ind_mod!(right, 3, 1, mods, Modifier::KC_LCTRL);
    at_ind_keycode!(right, 3, 2, keycodes, code_ind, KeyCode::KC_Q);
    at_ind_keycode!(right, 3, 3, keycodes, code_ind, KeyCode::KC_J);
    at_ind_keycode!(right, 3, 4, keycodes, code_ind, KeyCode::KC_K);

     */
    at_ind_keycode!(right, 3, 5, keycodes, code_ind, KeyCode::KC_SPC);

    LayerResult {
        next_layer: None,
        report: KeyboardReport {
            modifier: mods,
            reserved: 0,
            leds: 0,
            keycodes,
        },
    }
}

struct OccupiedSlot {
    code: KeyCode,
    dest: usize,
}

pub struct KeyboardReportState {
    inner_report: KeyboardReport,
    active_layer: KeymapLayer,
    cursor: usize,
    jank: JankState,
}

struct JankState {
    pressing_double_quote: bool,
    pressing_single_quote: bool,
    pressing_left_bracket: bool,
    pressing_comma: bool,
    pressing_right_bracket: bool,
    pressing_dot: bool,
    pressing_semicolon: bool,
    pressing_reg_colon: bool,
}

impl KeyboardReportState {

    pub fn new() -> Self {
        Self {
            inner_report: KeyboardReport {
                modifier: 0,
                reserved: 0,
                leds: 0,
                keycodes: [0u8; 6],
            },
            active_layer: KeymapLayer::DvorakSe,
            cursor: 0,
            jank: JankState {
                pressing_double_quote: false,
                pressing_single_quote: false,
                pressing_left_bracket: false,
                pressing_comma: false,
                pressing_right_bracket: false,
                pressing_dot: false,
                pressing_semicolon: false,
                pressing_reg_colon: false,
            },
        }
    }

    pub fn report(&self) -> &KeyboardReport {
        &self.inner_report
    }

    #[inline]
    fn push_key(&mut self, key_code: KeyCode) {
        // Don't know if there's ever a case where pressing more keys is valid, just replace front
        self.inner_report.keycodes[0] = key_code.0;
    }

    fn pop_key(&mut self, key_code: KeyCode) {
        if self.inner_report.keycodes[0] == key_code.0 {
            self.inner_report.keycodes[0] = 0;
        }
    }


    #[inline]
    fn push_modifier(&mut self, modifier: Modifier) {
        self.inner_report.modifier |= modifier.0
    }

    #[inline]
    fn pop_modifier(&mut self, modifier: Modifier) {
        self.inner_report.modifier &= !modifier.0
    }

    #[inline]
    fn has_modifier(&self, modifier: Modifier) -> bool {
        self.inner_report.modifier & modifier.0 != 0
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LeftSide;

#[derive(Copy, Clone, Debug)]
pub struct RightSide;

pub trait KeyboardSide {}

impl KeyboardSide for LeftSide {}
impl KeyboardSide for RightSide {}

pub trait KeyboardPosition {}

pub trait StateChangeHandler<S, R, C> where S: KeyboardSide {
}

pub trait KeyboardButton {
    #[inline(always)]
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {}
}

macro_rules! keyboard_key {
    ($($side: ident, $row: expr, $col: expr),*,) => {
        paste! {
            $(
                #[repr(transparent)]
                #[derive(Copy, Clone, Default)]
                pub struct [<$side Row $row Col $col>](bool);
                impl [<$side Row $row Col $col>] {
                    pub const ROW: u8 = $row;
                    pub const COL: u8 = $col;
                    pub const MATRIX_IND: usize = $crate::keyboard::matrix_ind($row, $col);
                }
            )*
        }
        paste! {
            pub struct KeyboardState {
                $(
                    [<$side:snake _ row $row _ col $col>]: [<$side Row $row Col $col>],
                )*
            }

            impl KeyboardState {
                pub const fn new() -> Self {
                    Self {
                        $(
                            [<$side:snake _ row $row _ col $col>]: [<$side Row $row Col $col>](false),
                        )*
                    }
                }
            }
        }
    };
}

pub trait ReadKeyPins {
    // Could clean up the codegen so that the right-side doesn't get invoked ever,
    // hopefully the compiler will just remove this entirely on those cases
    #[inline(always)]
    fn read_left_pins(&mut self, left_buttons: &mut LeftButtons) {

    }
}

keyboard_key!(
    Left, 0, 0,
    Left, 0, 1,
    Left, 0, 2,
    Left, 0, 3,
    Left, 0, 4,
    Left, 0, 5,

    Left, 1, 0,
    Left, 1, 1,
    Left, 1, 2,
    Left, 1, 3,
    Left, 1, 4,
    Left, 1, 5,

    Left, 2, 0,
    Left, 2, 1,
    Left, 2, 2,
    Left, 2, 3,
    Left, 2, 4,
    Left, 2, 5,

    Left, 3, 0,
    Left, 3, 1,
    Left, 3, 2,
    Left, 3, 3,
    Left, 3, 4,
    Left, 3, 5,

    Left, 4, 1,
    Left, 4, 2,
    Left, 4, 3,
    Left, 4, 4,
    Left, 4, 5,

    Right, 0, 0,
    Right, 0, 1,
    Right, 0, 2,
    Right, 0, 3,
    Right, 0, 4,
    Right, 0, 5,

    Right, 1, 0,
    Right, 1, 1,
    Right, 1, 2,
    Right, 1, 3,
    Right, 1, 4,
    Right, 1, 5,

    Right, 2, 0,
    Right, 2, 1,
    Right, 2, 2,
    Right, 2, 3,
    Right, 2, 4,
    Right, 2, 5,

    Right, 3, 0,
    Right, 3, 1,
    Right, 3, 2,
    Right, 3, 3,
    Right, 3, 4,
    Right, 3, 5,

    Right, 4, 0,
    Right, 4, 1,
    Right, 4, 2,
    Right, 4, 3,
    Right, 4, 4,
    Right, 4, 5,
);

macro_rules! pressed_push_pop_kc {
    ($state: expr, $pressed: expr, $kc: expr) => {
        {
            if $pressed {
                $state.push_key($kc);
            } else {
                $state.pop_key($kc);
            }
        }
    };
}

macro_rules! pressed_push_pop_modifier {
    ($state: expr, $pressed: expr, $modifier: expr) => {
        {
            if $pressed {
                $state.push_modifier($modifier);
            } else {
                $state.pop_modifier($modifier);
            }
        }
    };
}

macro_rules! bail_if_same {
    ($slf: expr, $pressed: expr) => {
        if $slf.0 == $pressed {
            return;
        }
    };
}

macro_rules! impl_read_pin_col {
    ($($arg: expr, $structure: expr, $row: tt,)*, $col: tt) => {
        paste! {
            pub fn [<read_col _ $col _pins>]($($arg: &mut $structure,)* left_buttons: &mut LeftButtons, keyboard_report_state: &mut KeyboardReportState) {
                let mut col = left_buttons.cols.$col.take().unwrap();
                let mut col = col.into_push_pull_output_in_state(PinState::Low);
                $(
                    $arg.update_state(matches!(left_buttons.rows[$row].is_low(), Ok(true)), keyboard_report_state);
                )*
                let _ = col.set_high();
                left_buttons.cols.$col = Some(col.into_pull_up_input());
            }

        }

    };
}

// Column pin gets toggled, more efficient to check all rows for each col at once
// Col 0 doesn't exist on row 4
impl_read_pin_col!(
    l00, LeftRow0Col0, 0,
    l01, LeftRow1Col0, 1,
    l02, LeftRow2Col0, 2,
    l03, LeftRow3Col0, 3,
    ,0
);

impl_read_pin_col!(
    l00, LeftRow0Col1, 0,
    l01, LeftRow1Col1, 1,
    l02, LeftRow2Col1, 2,
    l03, LeftRow3Col1, 3,
    l04, LeftRow4Col1, 4,
    ,1
);

impl_read_pin_col!(
    l00, LeftRow0Col2, 0,
    l01, LeftRow1Col2, 1,
    l02, LeftRow2Col2, 2,
    l03, LeftRow3Col2, 3,
    l04, LeftRow4Col2, 4,
    ,2
);

impl_read_pin_col!(
    l00, LeftRow0Col3, 0,
    l01, LeftRow1Col3, 1,
    l02, LeftRow2Col3, 2,
    l03, LeftRow3Col3, 3,
    l04, LeftRow4Col3, 4,
    ,3
);

impl_read_pin_col!(
    l00, LeftRow0Col4, 0,
    l01, LeftRow1Col4, 1,
    l02, LeftRow2Col4, 2,
    l03, LeftRow3Col4, 3,
    l04, LeftRow4Col4, 4,
    ,4
);

// Last row only has 5 pins
impl_read_pin_col!(
    l00, LeftRow0Col5, 0,
    l01, LeftRow1Col5, 1,
    l02, LeftRow2Col5, 2,
    l03, LeftRow3Col5, 3,
    l04, LeftRow4Col5, 4,
    ,5
);


impl KeyboardState {
    pub fn scan_left(&mut self, left_buttons: &mut LeftButtons, keyboard_report_state: &mut KeyboardReportState) {
        read_col_0_pins(&mut self.left_row0_col0, &mut self.left_row1_col0, &mut self.left_row2_col0, &mut self.left_row3_col0, left_buttons, keyboard_report_state);
        read_col_1_pins(&mut self.left_row0_col1, &mut self.left_row1_col1, &mut self.left_row2_col1, &mut self.left_row3_col1, &mut self.left_row4_col1, left_buttons, keyboard_report_state);
        read_col_2_pins(&mut self.left_row0_col2, &mut self.left_row1_col2, &mut self.left_row2_col2, &mut self.left_row3_col2, &mut self.left_row4_col2, left_buttons, keyboard_report_state);
        read_col_3_pins(&mut self.left_row0_col3, &mut self.left_row1_col3, &mut self.left_row2_col3, &mut self.left_row3_col3, &mut self.left_row4_col3, left_buttons, keyboard_report_state);
        read_col_4_pins(&mut self.left_row0_col4, &mut self.left_row1_col4, &mut self.left_row2_col4, &mut self.left_row3_col4, &mut self.left_row4_col4, left_buttons, keyboard_report_state);
        read_col_5_pins(&mut self.left_row0_col5, &mut self.left_row1_col5, &mut self.left_row2_col5, &mut self.left_row3_col5, &mut self.left_row4_col5, left_buttons, keyboard_report_state);
    }
}
impl KeyboardButton for LeftRow0Col0 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::KC_TAB);
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow0Col1 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::COMMA);
            }
            KeymapLayer::DvorakSe => {
                if pressed {
                    if keyboard_report_state.has_modifier(Modifier::LEFT_SHIFT) {
                        // Shifted, `SHIFT + 2` -> "
                        keyboard_report_state.jank.pressing_double_quote = true;
                        keyboard_report_state.push_key(KeyCode::N2);
                    } else {
                        // Not shifted, \ -> '
                        keyboard_report_state.jank.pressing_single_quote = true;
                        keyboard_report_state.push_key(KeyCode::BACKSLASH);
                    }
                } else {
                    if keyboard_report_state.jank.pressing_double_quote {
                        keyboard_report_state.pop_key(KeyCode::N2);
                        keyboard_report_state.jank.pressing_double_quote = false;
                    }
                    if keyboard_report_state.jank.pressing_single_quote {
                        keyboard_report_state.jank.pressing_single_quote = false;
                        keyboard_report_state.pop_key(KeyCode::BACKSLASH);
                    }
                }
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow0Col2 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::COMMA);
            }
            KeymapLayer::DvorakSe => {
                if pressed {
                    if keyboard_report_state.has_modifier(Modifier::LEFT_SHIFT) {
                        // Need to remove shift for this key to go out, not putting it
                        // back after though for reasons that I don't remember and may be a bug
                        keyboard_report_state.pop_modifier(Modifier::LEFT_SHIFT);
                        keyboard_report_state.push_key(KeyCode::NON_US_BACKSLASH);
                        keyboard_report_state.jank.pressing_left_bracket = true;
                    } else {
                        keyboard_report_state.push_key(KeyCode::COMMA);
                        keyboard_report_state.jank.pressing_comma = true;
                    }
                } else {
                    if (keyboard_report_state.jank.pressing_left_bracket) {
                        keyboard_report_state.pop_key(KeyCode::NON_US_BACKSLASH);
                        keyboard_report_state.push_modifier(Modifier::LEFT_SHIFT);
                        keyboard_report_state.jank.pressing_left_bracket = false;
                    }
                    if (keyboard_report_state.jank.pressing_comma) {
                        keyboard_report_state.pop_key(KeyCode::COMMA);
                        keyboard_report_state.jank.pressing_comma = false;
                    }
                }
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow0Col3 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::DOT);
            }
            KeymapLayer::DvorakSe => {
                if pressed {
                    if keyboard_report_state.has_modifier(Modifier::LEFT_SHIFT) {
                        // Needs a shift, but that's already pressed
                        keyboard_report_state.push_key(KeyCode::NON_US_BACKSLASH);
                        keyboard_report_state.jank.pressing_right_bracket = true;
                    } else {
                        keyboard_report_state.push_key(KeyCode::DOT);
                        keyboard_report_state.jank.pressing_dot = true;
                    }
                } else {
                    if keyboard_report_state.jank.pressing_right_bracket {
                        keyboard_report_state.pop_key(KeyCode::NON_US_BACKSLASH);
                        keyboard_report_state.jank.pressing_right_bracket = false;
                    }
                    if keyboard_report_state.jank.pressing_dot {
                        keyboard_report_state.pop_key(KeyCode::DOT);
                        keyboard_report_state.jank.pressing_dot = false;
                    }
                }
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow0Col4 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::P)
            }
        }
        self.0 = pressed;
    }
}
impl KeyboardButton for LeftRow0Col5 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::Y)
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow1Col0 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::ESCAPE);
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow1Col1 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::A);
            }
        }
        self.0 = pressed;
    }
}
impl KeyboardButton for LeftRow1Col2 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::O);
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow1Col3 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::E);
            }
        }
        self.0 = pressed;
    }
}
impl KeyboardButton for LeftRow1Col4 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::U);
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow1Col5 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::I);
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow2Col0 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        pressed_push_pop_modifier!(keyboard_report_state, pressed, Modifier::LEFT_SHIFT);
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow2Col1 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::SEMICOLON);
            }
            KeymapLayer::DvorakSe => {
                if pressed {
                    if keyboard_report_state.has_modifier(Modifier::LEFT_SHIFT) {
                        // Needs a shift, but that's already pressed
                        keyboard_report_state.push_key(KeyCode::DOT);
                        keyboard_report_state.jank.pressing_reg_colon = true;
                    } else {
                        keyboard_report_state.push_modifier(Modifier::LEFT_SHIFT);
                        keyboard_report_state.push_key(KeyCode::COMMA);
                        keyboard_report_state.jank.pressing_semicolon = true;
                    }
                } else {
                    if keyboard_report_state.jank.pressing_reg_colon {
                        keyboard_report_state.pop_key(KeyCode::DOT);
                        keyboard_report_state.jank.pressing_reg_colon = false;
                    }
                    if keyboard_report_state.jank.pressing_semicolon {
                        keyboard_report_state.pop_key(KeyCode::COMMA);
                        keyboard_report_state.pop_modifier(Modifier::LEFT_SHIFT);
                        keyboard_report_state.jank.pressing_semicolon = false;
                    }
                }
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow2Col2 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::Q);
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow2Col3 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::J);
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow2Col4 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::K);
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow2Col5 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::X);
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow3Col0 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        pressed_push_pop_modifier!(keyboard_report_state, pressed, Modifier::LEFT_CONTROL);
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow3Col1 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_modifier!(keyboard_report_state, pressed, Modifier::LEFT_GUI);
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow3Col2 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi |
            KeymapLayer::DvorakSe => {
                pressed_push_pop_modifier!(keyboard_report_state, pressed, Modifier::LEFT_ALT);
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow3Col3 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        if pressed {
            reset_to_usb_boot(0, 0);
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow3Col4 {

}

impl KeyboardButton for LeftRow3Col5 {

}


impl KeyboardButton for LeftRow4Col1 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        if pressed {
            reset_to_usb_boot(0, 0);
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow4Col2 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        if pressed {
            match keyboard_report_state.active_layer {
                KeymapLayer::DvorakAnsi => {
                    keyboard_report_state.active_layer = KeymapLayer::DvorakSe;
                }
                KeymapLayer::DvorakSe => {
                    keyboard_report_state.active_layer = KeymapLayer::DvorakAnsi;
                }
            }
        }
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow4Col3 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::N3);
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow4Col4 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::N4);
        self.0 = pressed;
    }
}

impl KeyboardButton for LeftRow4Col5 {
    fn update_state(&mut self, pressed: bool, keyboard_report_state: &mut KeyboardReportState) {
        bail_if_same!(self, pressed);
        pressed_push_pop_kc!(keyboard_report_state, pressed, KeyCode::N5);
        self.0 = pressed;
    }
}