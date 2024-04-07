#[cfg(feature = "serial")]
use core::fmt::Write;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use paste::paste;
use rp2040_hal::gpio::PinState;
use rp2040_hal::rom_data::reset_to_usb_boot;
use rp2040_hal::Timer;
use rp2040_kbd_lib::queue::Queue;
use usbd_hid::descriptor::KeyboardReport;

use crate::hid::keycodes::{KeyCode, Modifier};
use crate::keyboard::jitter::JitterRegulator;
use crate::keyboard::left::LeftButtons;
use crate::keyboard::{MatrixChange, MatrixUpdate};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum KeymapLayer {
    DvorakSe,
    DvorakAnsi,
    QwertyAnsi,
    QwertyGaming,
    Lower,
    LowerAnsi,
    Raise,
    Num,
    Settings,
}

fn copy_report(keyboard_report: &KeyboardReport) -> KeyboardReport {
    KeyboardReport {
        modifier: keyboard_report.modifier,
        reserved: keyboard_report.reserved,
        leds: keyboard_report.leds,
        keycodes: keyboard_report.keycodes,
    }
}

pub struct KeyboardReportState {
    inner_report: KeyboardReport,
    temp_mods: Option<Modifier>,
    seq: usize,
    outbound_reports: Queue<KeyboardReport, 16>,
    active_layer: KeymapLayer,
    last_perm_layer: Option<KeymapLayer>,
    jank: JankState,
    layer_change: Option<KeymapLayer>,
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
    autoshifted: bool,
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
            temp_mods: None,
            seq: 0,
            outbound_reports: Queue::new(),
            active_layer: KeymapLayer::DvorakSe,
            last_perm_layer: None,
            jank: JankState {
                pressing_double_quote: false,
                pressing_single_quote: false,
                pressing_left_bracket: false,
                pressing_comma: false,
                pressing_right_bracket: false,
                pressing_dot: false,
                pressing_semicolon: false,
                pressing_reg_colon: false,
                autoshifted: false,
            },
            layer_change: Some(KeymapLayer::DvorakSe),
        }
    }

    #[cfg(feature = "hiddev")]
    pub fn report(&self) -> Option<&KeyboardReport> {
        self.outbound_reports.peek()
    }

    #[cfg(feature = "hiddev")]
    pub fn accept(&mut self) {
        self.outbound_reports.pop_front();
    }

    pub fn layer_update(&mut self) -> Option<KeymapLayer> {
        self.layer_change.take()
    }

    fn next_seq(&mut self) -> usize {
        let old = self.seq;
        self.seq = self.seq.wrapping_add(1);
        old
    }

    #[inline]
    fn push_key(&mut self, key_code: KeyCode) {
        // Don't know if there's ever a case where pressing more keys is valid, just replace front
        self.inner_report.keycodes[0] = key_code.0;
        self.outbound_reports
            .push_back(copy_report(&self.inner_report));
    }

    fn pop_key(&mut self, key_code: KeyCode) {
        if self.inner_report.keycodes[0] == key_code.0 {
            self.inner_report.keycodes[0] = 0;
            self.outbound_reports
                .push_back(copy_report(&self.inner_report));
        }
    }

    #[inline]
    fn push_modifier(&mut self, modifier: Modifier) {
        self.inner_report.modifier |= modifier.0;
        self.outbound_reports
            .push_back(copy_report(&self.inner_report));
    }

    #[inline]
    fn pop_modifier(&mut self, modifier: Modifier) {
        self.inner_report.modifier &= !modifier.0;
        self.outbound_reports
            .push_back(copy_report(&self.inner_report));
    }

    #[inline]
    fn has_modifier(&self, modifier: Modifier) -> bool {
        self.inner_report.modifier & modifier.0 != 0
    }

    #[inline]
    fn push_layer_with_fallback(&mut self, keymap_layer: KeymapLayer) {
        self.last_perm_layer = Some(core::mem::replace(&mut self.active_layer, keymap_layer));
        self.layer_change = Some(self.active_layer);
    }

    #[inline]
    fn pop_layer(&mut self, this: KeymapLayer) {
        if self.active_layer == this {
            if let Some(old) = self.last_perm_layer.take() {
                self.active_layer = old;
                self.layer_change = Some(self.active_layer);
            }
        }
    }

    #[inline]
    fn set_perm_layer(&mut self, keymap_layer: KeymapLayer) {
        self.active_layer = keymap_layer;
        self.layer_change = Some(keymap_layer);
        self.last_perm_layer = None;
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

pub trait StateChangeHandler<S, R, C>
where
    S: KeyboardSide,
{
}

pub trait KeyboardButton {
    #[inline(always)]
    fn update_state(
        &mut self,
        _pressed: bool,
        _keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        false
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LastPressState {
    seq: usize,
    layer: KeymapLayer,
}

pub struct PinStructState {
    last_state: Option<LastPressState>,
    jitter: JitterRegulator,
}

impl PinStructState {
    #[inline]
    fn is_pressed(&self) -> bool {
        self.last_state.is_some()
    }

    fn update_last_state(&mut self, seq: usize, layer: KeymapLayer) {
        self.last_state = Some(LastPressState {
            seq,
            layer,
        });
    }
}

impl PinStructState {
    pub const fn new() -> Self {
        Self {
            last_state: None,
            jitter: JitterRegulator::new(),
        }
    }
}

macro_rules! keyboard_key {
    ($($side: ident, $row: expr, $col: expr),*,) => {
        paste! {
            $(
                #[repr(transparent)]
                pub struct [<$side Row $row Col $col>](PinStructState);

                impl [<$side Row $row Col $col>] {
                    pub const fn new() -> Self {
                        Self(PinStructState::new())
                    }

                    #[allow(dead_code)]
                    pub fn check_update_state(
                        &mut self,
                        pressed: bool,
                        keyboard_report_state: &mut KeyboardReportState,
                        timer: Timer,
                    ) -> bool {
                        if pressed != self.0.is_pressed() {
                            if self.0.jitter.try_submit(timer.get_counter(), pressed) {
                                self.update_state(pressed, keyboard_report_state);
                                return true;
                            }
                        }
                        false
                    }

                    #[allow(dead_code)]
                    pub fn check_jitter_state(
                        &mut self,
                        keyboard_report_state: &mut KeyboardReportState,
                        timer: Timer,
                    ) -> bool {
                        let mut any_change = false;
                        if let Some(next) = self.0.jitter.try_release_quarantined(timer.get_counter()) {
                            if self.0.is_pressed() != next {
                                self.update_state(next, keyboard_report_state);
                                any_change = true;
                            }
                        }
                        any_change
                    }
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
                            [<$side:snake _ row $row _ col $col>]: [<$side Row $row Col $col>]::new(),
                        )*
                    }
                }
            }
        }
    };
}

keyboard_key!(
    Left, 0, 0, Left, 0, 1, Left, 0, 2, Left, 0, 3, Left, 0, 4, Left, 0, 5, Left, 1, 0, Left, 1, 1,
    Left, 1, 2, Left, 1, 3, Left, 1, 4, Left, 1, 5, Left, 2, 0, Left, 2, 1, Left, 2, 2, Left, 2, 3,
    Left, 2, 4, Left, 2, 5, Left, 3, 0, Left, 3, 1, Left, 3, 2, Left, 3, 3, Left, 3, 4, Left, 3, 5,
    Left, 4, 1, Left, 4, 2, Left, 4, 3, Left, 4, 4, Left, 4, 5, Right, 0, 0, Right, 0, 1, Right, 0,
    2, Right, 0, 3, Right, 0, 4, Right, 0, 5, Right, 1, 0, Right, 1, 1, Right, 1, 2, Right, 1, 3,
    Right, 1, 4, Right, 1, 5, Right, 2, 0, Right, 2, 1, Right, 2, 2, Right, 2, 3, Right, 2, 4,
    Right, 2, 5, Right, 3, 0, Right, 3, 1, Right, 3, 2, Right, 3, 3, Right, 3, 4, Right, 3, 5,
    Right, 4, 1, Right, 4, 2, Right, 4, 3, Right, 4, 4, Right, 4, 5,
);

macro_rules! impl_read_pin_col {
    ($($structure: expr, $row: tt,)*, $col: tt) => {
        paste! {
            pub fn [<read_col _ $col _pins>]($([< $structure:snake >]: &mut $structure,)* left_buttons: &mut LeftButtons, keyboard_report_state: &mut KeyboardReportState, timer: Timer) -> bool {
                let col = left_buttons.cols.$col.take().unwrap();
                let mut col = col.into_push_pull_output_in_state(PinState::Low);
                let mut any_change = false;
                $(
                    if [< $structure:snake >].check_update_state(matches!(left_buttons.rows[$row].is_low(), Ok(true)), keyboard_report_state, timer) {
                        any_change = true;
                    } else if [< $structure:snake >].check_jitter_state(keyboard_report_state, timer) {
                        any_change = true;
                    }
                )*
                let _ = col.set_high();
                left_buttons.cols.$col = Some(col.into_pull_up_input());
                any_change
            }

        }

    };
}

// Column pin gets toggled, more efficient to check all rows for each col at once
// Col 0 doesn't exist on row 4
impl_read_pin_col!(
    LeftRow0Col0, 0,
    LeftRow1Col0, 1,
    LeftRow2Col0, 2,
    LeftRow3Col0, 3,
    ,0
);

impl_read_pin_col!(
    LeftRow0Col1, 0,
    LeftRow1Col1, 1,
    LeftRow2Col1, 2,
    LeftRow3Col1, 3,
    LeftRow4Col1, 4,
    ,1
);

impl_read_pin_col!(
    LeftRow0Col2, 0,
    LeftRow1Col2, 1,
    LeftRow2Col2, 2,
    LeftRow3Col2, 3,
    LeftRow4Col2, 4,
    ,2
);

impl_read_pin_col!(
    LeftRow0Col3, 0,
    LeftRow1Col3, 1,
    LeftRow2Col3, 2,
    LeftRow3Col3, 3,
    LeftRow4Col3, 4,
    ,3
);

impl_read_pin_col!(
    LeftRow0Col4, 0,
    LeftRow1Col4, 1,
    LeftRow2Col4, 2,
    LeftRow3Col4, 3,
    LeftRow4Col4, 4,
    ,4
);

// Last row only has 5 pins
impl_read_pin_col!(
    LeftRow0Col5, 0,
    LeftRow1Col5, 1,
    LeftRow2Col5, 2,
    LeftRow3Col5, 3,
    LeftRow4Col5, 4,
    ,5
);

impl KeyboardState {
    pub fn scan_left(
        &mut self,
        left_buttons: &mut LeftButtons,
        keyboard_report_state: &mut KeyboardReportState,
        timer: Timer,
    ) -> bool {
        let col0_change = read_col_0_pins(
            &mut self.left_row0_col0,
            &mut self.left_row1_col0,
            &mut self.left_row2_col0,
            &mut self.left_row3_col0,
            left_buttons,
            keyboard_report_state,
            timer,
        );
        let col1_change = read_col_1_pins(
            &mut self.left_row0_col1,
            &mut self.left_row1_col1,
            &mut self.left_row2_col1,
            &mut self.left_row3_col1,
            &mut self.left_row4_col1,
            left_buttons,
            keyboard_report_state,
            timer,
        );
        let col2_change = read_col_2_pins(
            &mut self.left_row0_col2,
            &mut self.left_row1_col2,
            &mut self.left_row2_col2,
            &mut self.left_row3_col2,
            &mut self.left_row4_col2,
            left_buttons,
            keyboard_report_state,
            timer,
        );
        let col3_change = read_col_3_pins(
            &mut self.left_row0_col3,
            &mut self.left_row1_col3,
            &mut self.left_row2_col3,
            &mut self.left_row3_col3,
            &mut self.left_row4_col3,
            left_buttons,
            keyboard_report_state,
            timer,
        );
        let col4_change = read_col_4_pins(
            &mut self.left_row0_col4,
            &mut self.left_row1_col4,
            &mut self.left_row2_col4,
            &mut self.left_row3_col4,
            &mut self.left_row4_col4,
            left_buttons,
            keyboard_report_state,
            timer,
        );
        let col5_change = read_col_5_pins(
            &mut self.left_row0_col5,
            &mut self.left_row1_col5,
            &mut self.left_row2_col5,
            &mut self.left_row3_col5,
            &mut self.left_row4_col5,
            left_buttons,
            keyboard_report_state,
            timer,
        );
        col0_change || col1_change || col2_change || col3_change || col4_change || col5_change
    }

    pub fn update_right(
        &mut self,
        update: MatrixUpdate,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match update.matrix_change() {
            MatrixChange::Encoder(enc) => {
                rotate_layer(enc, keyboard_report_state);
            }
            MatrixChange::Key(ind, change) => {
                #[cfg(feature = "serial")]
                {
                    let (row, col) = crate::keyboard::matrix_ind_to_row_col(ind as usize);
                    let _ = core::fmt::Write::write_fmt(
                        &mut crate::runtime::shared::usb::acquire_usb(),
                        format_args!("R: R{row}, C{col} -> {}\r\n", change),
                    );
                }
                match ind {
                    0 => {
                        self.right_row0_col0
                            .update_state(change, keyboard_report_state);
                    }
                    1 => {
                        self.right_row0_col1
                            .update_state(change, keyboard_report_state);
                    }
                    2 => {
                        self.right_row0_col2
                            .update_state(change, keyboard_report_state);
                    }
                    3 => {
                        self.right_row0_col3
                            .update_state(change, keyboard_report_state);
                    }
                    4 => {
                        self.right_row0_col4
                            .update_state(change, keyboard_report_state);
                    }
                    5 => {
                        self.right_row0_col5
                            .update_state(change, keyboard_report_state);
                    }
                    6 => {
                        self.right_row1_col0
                            .update_state(change, keyboard_report_state);
                    }
                    7 => {
                        self.right_row1_col1
                            .update_state(change, keyboard_report_state);
                    }
                    8 => {
                        self.right_row1_col2
                            .update_state(change, keyboard_report_state);
                    }
                    9 => {
                        self.right_row1_col3
                            .update_state(change, keyboard_report_state);
                    }
                    10 => {
                        self.right_row1_col4
                            .update_state(change, keyboard_report_state);
                    }
                    11 => {
                        self.right_row1_col5
                            .update_state(change, keyboard_report_state);
                    }
                    12 => {
                        self.right_row2_col0
                            .update_state(change, keyboard_report_state);
                    }
                    13 => {
                        self.right_row2_col1
                            .update_state(change, keyboard_report_state);
                    }
                    14 => {
                        self.right_row2_col2
                            .update_state(change, keyboard_report_state);
                    }
                    15 => {
                        self.right_row2_col3
                            .update_state(change, keyboard_report_state);
                    }
                    16 => {
                        self.right_row2_col4
                            .update_state(change, keyboard_report_state);
                    }
                    17 => {
                        self.right_row2_col5
                            .update_state(change, keyboard_report_state);
                    }
                    18 => {
                        self.right_row3_col0
                            .update_state(change, keyboard_report_state);
                    }
                    19 => {
                        self.right_row3_col1
                            .update_state(change, keyboard_report_state);
                    }
                    20 => {
                        self.right_row3_col2
                            .update_state(change, keyboard_report_state);
                    }
                    21 => {
                        self.right_row3_col3
                            .update_state(change, keyboard_report_state);
                    }
                    22 => {
                        self.right_row3_col4
                            .update_state(change, keyboard_report_state);
                    }
                    23 => {
                        self.right_row3_col5
                            .update_state(change, keyboard_report_state);
                    }
                    25 => {
                        self.right_row4_col1
                            .update_state(change, keyboard_report_state);
                    }
                    26 => {
                        self.right_row4_col2
                            .update_state(change, keyboard_report_state);
                    }
                    27 => {
                        self.right_row4_col3
                            .update_state(change, keyboard_report_state);
                    }
                    28 => {
                        self.right_row4_col4
                            .update_state(change, keyboard_report_state);
                    }
                    29 => {
                        self.right_row4_col5
                            .update_state(change, keyboard_report_state);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn rotate_layer(clockwise: bool, keyboard_report_state: &mut KeyboardReportState) {
    match (
        keyboard_report_state.active_layer,
        keyboard_report_state.last_perm_layer,
    ) {
        (KeymapLayer::DvorakSe, _) => {
            if clockwise {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakAnsi);
            } else {
                keyboard_report_state.set_perm_layer(KeymapLayer::QwertyGaming);
            }
        }
        (KeymapLayer::DvorakAnsi, _) => {
            if clockwise {
                keyboard_report_state.set_perm_layer(KeymapLayer::QwertyAnsi);
            } else {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakSe);
            }
        }
        (KeymapLayer::QwertyAnsi, _) => {
            if clockwise {
                keyboard_report_state.set_perm_layer(KeymapLayer::QwertyGaming);
            } else {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakAnsi);
            }
        }
        (KeymapLayer::QwertyGaming, _) => {
            if clockwise {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakSe);
            } else {
                keyboard_report_state.set_perm_layer(KeymapLayer::QwertyAnsi);
            }
        }
        _ => {}
    }
    #[cfg(feature = "serial")]
    {
        let _ = crate::runtime::shared::usb::acquire_usb().write_fmt(format_args!(
            "Post rotate layer: {:?}\r\n",
            keyboard_report_state.active_layer
        ));
    }
}

macro_rules! simple_push_pop_kc {
    ($slf: expr, $state: expr, $kc: expr) => {{
        if let Some(_next) = $slf.0.last_state.take() {
            $state.pop_key($kc);
        } else {
            $state.push_key($kc);
        }
    }};
}

macro_rules! simple_push_pop_mod {
    ($slf: expr, $state: expr, $modifier: expr) => {{
        if let Some(_next) = $slf.0.last_state.take() {
            $state.pop_modifier($modifier);
        } else {
            $state.push_modifier($modifier);
        }
    }};
}

macro_rules! autoshift_kc {
    ($slf: expr, $state: expr, $kc: expr) => {
        if let Some(prev) = $slf.0.last_state.take() {
            if $state.seq == prev.seq + 1 {
                $state.pop_modifier(Modifier::LEFT_SHIFT);
            }
            $state.pop_key($kc);
        } else {
            if $state.has_modifier(Modifier::ANY_SHIFT) {
                $state.push_key($kc);
            } else {
                $state.push_key($kc);
                $state.push_modifier(Modifier::LEFT_SHIFT);
            }
        }
    };
}

macro_rules! with_modifier_kc {
    ($slf: expr, $state: expr, $modifier: expr, $kc: expr) => {
        if let Some(prev) = $slf.0.last_state.take() {
            if $state.seq == prev.seq + 1 {
                $state.pop_modifier($modifier);
            }
            $state.pop_key($kc);
        } else {
            $state.push_modifier($modifier);
            $state.push_key($kc);
        }
    };
}

impl KeyboardButton for LeftRow0Col0 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        simple_push_pop_kc!(self, keyboard_report_state, KeyCode::KC_TAB);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow0Col1 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::COMMA);
            }
            KeymapLayer::DvorakSe => {
                if pressed {
                    if keyboard_report_state.has_modifier(Modifier::ANY_SHIFT) {
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
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::Q);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N1);
            }
            KeymapLayer::Lower | KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N1);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F1);
            }
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow0Col2 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::COMMA);
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
                    if keyboard_report_state.jank.pressing_left_bracket {
                        keyboard_report_state.pop_key(KeyCode::NON_US_BACKSLASH);
                        keyboard_report_state.push_modifier(Modifier::LEFT_SHIFT);
                        keyboard_report_state.jank.pressing_left_bracket = false;
                    }
                    if keyboard_report_state.jank.pressing_comma {
                        keyboard_report_state.pop_key(KeyCode::COMMA);
                        keyboard_report_state.jank.pressing_comma = false;
                    }
                }
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::W);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N2);
            }
            KeymapLayer::Lower => {
                with_modifier_kc!(
                    self,
                    keyboard_report_state,
                    Modifier::RIGHT_ALT,
                    KeyCode::N2
                );
            }
            KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N2)
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F2)
            }
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow0Col3 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::DOT);
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
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::E);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N3);
            }
            KeymapLayer::Lower | KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N3);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F3);
            }
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow0Col4 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::P)
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::R);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N4);
            }
            KeymapLayer::Lower => {
                with_modifier_kc!(
                    self,
                    keyboard_report_state,
                    Modifier::RIGHT_ALT,
                    KeyCode::N4
                );
            }
            KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N4);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F4);
            }
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}
impl KeyboardButton for LeftRow0Col5 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::Y)
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::T);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N5);
            }
            KeymapLayer::Lower | KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N5);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F4);
            }
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow1Col0 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        simple_push_pop_kc!(self, keyboard_report_state, KeyCode::ESCAPE);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow1Col1 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe | KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::A);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::T);
            }
            KeymapLayer::Lower => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::DASH)
            }
            KeymapLayer::LowerAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::KP_PLUS);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::LEFT_ARROW);
            }
            KeymapLayer::Num => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N1);
            }
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}
impl KeyboardButton for LeftRow1Col2 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::O);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::S);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::Q);
            }
            KeymapLayer::Lower => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N0);
            }
            KeymapLayer::LowerAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::EQUALS);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::RIGHT_ARROW);
            }
            KeymapLayer::Num => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N2);
            }
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow1Col3 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::E);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::D);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::W);
            }
            KeymapLayer::Lower => {
                with_modifier_kc!(
                    self,
                    keyboard_report_state,
                    Modifier::RIGHT_ALT,
                    KeyCode::N8
                );
            }
            KeymapLayer::LowerAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::LEFT_BRACKET);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::UP_ARROW);
            }
            KeymapLayer::Num => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N3);
            }
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}
impl KeyboardButton for LeftRow1Col4 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::U);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::E);
            }
            KeymapLayer::Lower => {
                with_modifier_kc!(
                    self,
                    keyboard_report_state,
                    Modifier::RIGHT_ALT,
                    KeyCode::N9
                );
            }
            KeymapLayer::LowerAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::RIGHT_BRACKET);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::DOWN_ARROW);
            }
            KeymapLayer::Num => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N4);
            }
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow1Col5 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::I);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::G);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::R);
            }
            KeymapLayer::Lower => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::DASH);
            }
            KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::SLASH);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F11);
            }
            KeymapLayer::Num => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N5);
            }
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow2Col0 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        simple_push_pop_mod!(self, keyboard_report_state, Modifier::LEFT_SHIFT);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow2Col1 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::SEMICOLON);
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
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::Z);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::Y);
            }
            KeymapLayer::Lower | KeymapLayer::LowerAnsi | KeymapLayer::Raise | KeymapLayer::Num => {
            }
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow2Col2 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::Q);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::X);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::A);
            }
            KeymapLayer::Lower | KeymapLayer::LowerAnsi => {
                with_modifier_kc!(
                    self,
                    keyboard_report_state,
                    Modifier::LEFT_CONTROL,
                    KeyCode::C
                );
            }
            KeymapLayer::Raise | KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow2Col3 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::J);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::C);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::S);
            }
            KeymapLayer::Lower | KeymapLayer::LowerAnsi => {
                with_modifier_kc!(
                    self,
                    keyboard_report_state,
                    Modifier::LEFT_CONTROL,
                    KeyCode::X
                );
            }
            KeymapLayer::Raise | KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow2Col4 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::K);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::V);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::D);
            }
            KeymapLayer::Lower | KeymapLayer::LowerAnsi => {
                with_modifier_kc!(
                    self,
                    keyboard_report_state,
                    Modifier::LEFT_CONTROL,
                    KeyCode::V
                );
            }
            KeymapLayer::Raise | KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow2Col5 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::X);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::B);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F);
            }
            KeymapLayer::Lower => {
                // ~ Tilde double-tap to get it out immediately
                if pressed {
                    keyboard_report_state.push_modifier(Modifier::RIGHT_ALT);
                    keyboard_report_state.push_key(KeyCode::RIGHT_BRACKET);
                    keyboard_report_state.pop_key(KeyCode::RIGHT_BRACKET);
                    keyboard_report_state.push_key(KeyCode::RIGHT_BRACKET);
                } else {
                    keyboard_report_state.pop_modifier(Modifier::RIGHT_ALT);
                    keyboard_report_state.pop_key(KeyCode::RIGHT_BRACKET);
                }
            }
            KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::GRAVE);
            }
            KeymapLayer::Raise | KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow3Col0 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        simple_push_pop_mod!(self, keyboard_report_state, Modifier::LEFT_CONTROL);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow3Col1 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe | KeymapLayer::QwertyAnsi => {
                simple_push_pop_mod!(self, keyboard_report_state, Modifier::LEFT_GUI);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::Z);
            }
            KeymapLayer::Lower | KeymapLayer::LowerAnsi | KeymapLayer::Raise | KeymapLayer::Num => {
            }
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow3Col2 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe | KeymapLayer::QwertyGaming => {
                simple_push_pop_mod!(self, keyboard_report_state, Modifier::LEFT_ALT);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::X);
            }
            KeymapLayer::Lower => {}
            KeymapLayer::LowerAnsi => {}
            KeymapLayer::Raise => {}
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow3Col3 {}

impl KeyboardButton for LeftRow3Col4 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::QwertyAnsi => {
                if pressed {
                    keyboard_report_state.push_layer_with_fallback(KeymapLayer::LowerAnsi);
                }
            }
            KeymapLayer::DvorakSe => {
                if pressed {
                    keyboard_report_state.push_layer_with_fallback(KeymapLayer::Lower);
                }
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::C);
            }
            KeymapLayer::Lower => {
                if !pressed {
                    keyboard_report_state.pop_layer(KeymapLayer::Lower);
                }
            }
            KeymapLayer::LowerAnsi => {
                if !pressed {
                    keyboard_report_state.pop_layer(KeymapLayer::LowerAnsi);
                }
            }
            KeymapLayer::Raise => {}
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow3Col5 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::SPACE);
            }
            KeymapLayer::QwertyAnsi => {}
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::C);
            }
            KeymapLayer::Lower => {}
            KeymapLayer::LowerAnsi => {}
            KeymapLayer::Raise => {}
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

// Row 4 col 0 does not exist
impl KeyboardButton for LeftRow4Col1 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        if pressed {
            reset_to_usb_boot(0, 0);
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow4Col2 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakSe => {}
            KeymapLayer::DvorakAnsi => {}
            KeymapLayer::QwertyAnsi => {}
            KeymapLayer::QwertyGaming => {
                simple_push_pop_mod!(self, keyboard_report_state, Modifier::LEFT_GUI);
            }
            KeymapLayer::Lower => {}
            KeymapLayer::LowerAnsi => {}
            KeymapLayer::Raise => {}
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow4Col3 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakSe | KeymapLayer::DvorakAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::DASH);
            }
            KeymapLayer::QwertyAnsi => {}
            KeymapLayer::QwertyGaming => {
                simple_push_pop_mod!(self, keyboard_report_state, Modifier::LEFT_ALT);
            }
            KeymapLayer::Lower => {}
            KeymapLayer::LowerAnsi => {}
            KeymapLayer::Raise => {}
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N3);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow4Col4 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakSe | KeymapLayer::DvorakAnsi => {}
            KeymapLayer::QwertyAnsi => {}
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::SPACE);
            }
            KeymapLayer::Lower => {}
            KeymapLayer::LowerAnsi => {}
            KeymapLayer::Raise => {}
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for LeftRow4Col5 {
    fn update_state(
        &mut self,
        _pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

// Right side, goes from right to left

impl KeyboardButton for RightRow0Col0 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        if let Some(_last) = self.0.last_state.take() {

        } else {

        }
        simple_push_pop_kc!(self, keyboard_report_state, KeyCode::BACKSPACE);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow0Col1 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::L);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::P);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N0);
            }
            KeymapLayer::Lower => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::BACKSLASH);
            }
            KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N8);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F10);
            }
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow0Col2 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::R);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::O);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N9);
            }
            KeymapLayer::Lower => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N9);
            }
            KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N0);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F9);
            }
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow0Col3 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::C);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::I);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N8);
            }
            KeymapLayer::Lower => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N8);
            }
            KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N9);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F8);
            }
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow0Col4 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::G);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::U);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N7);
            }
            KeymapLayer::Lower => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N6);
            }
            KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N7);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F7);
            }
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow0Col5 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F);
            }
            KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::Y);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N6);
            }
            KeymapLayer::Lower => {
                // Double-tap to get ^ on one press, not like I ever use circ for anything else
                if pressed {
                    if keyboard_report_state.has_modifier(Modifier::ANY_SHIFT) {
                        keyboard_report_state.push_key(KeyCode::RIGHT_BRACKET);
                        keyboard_report_state.pop_key(KeyCode::RIGHT_BRACKET);
                        keyboard_report_state.push_key(KeyCode::RIGHT_BRACKET);
                    } else {
                        keyboard_report_state.push_modifier(Modifier::LEFT_SHIFT);
                        keyboard_report_state.push_key(KeyCode::RIGHT_BRACKET);
                        keyboard_report_state.pop_key(KeyCode::RIGHT_BRACKET);
                        keyboard_report_state.push_key(KeyCode::RIGHT_BRACKET);
                        keyboard_report_state.jank.autoshifted = true;
                    }
                } else {
                    if keyboard_report_state.jank.autoshifted {
                        keyboard_report_state.pop_modifier(Modifier::LEFT_SHIFT);
                        keyboard_report_state.jank.autoshifted = false;
                    }
                    keyboard_report_state.pop_key(KeyCode::RIGHT_BRACKET);
                }
            }
            KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N6);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F6);
            }
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow1Col0 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        simple_push_pop_kc!(self, keyboard_report_state, KeyCode::ENTER);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow1Col1 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::S);
            }
            KeymapLayer::QwertyAnsi | KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::SEMICOLON);
            }
            KeymapLayer::Lower => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::SLASH);
            }
            KeymapLayer::LowerAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::DASH);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::KC_DELF);
            }
            KeymapLayer::Num => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N0);
            }
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow1Col2 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N);
            }
            KeymapLayer::QwertyAnsi | KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::L);
            }
            KeymapLayer::Lower => {
                with_modifier_kc!(
                    self,
                    keyboard_report_state,
                    Modifier::RIGHT_ALT,
                    KeyCode::N0
                );
            }
            KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::RIGHT_BRACKET);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::HOME);
            }
            KeymapLayer::Num => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N9);
            }
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow1Col3 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::T);
            }
            KeymapLayer::QwertyAnsi | KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::K);
            }
            KeymapLayer::Lower => {
                with_modifier_kc!(
                    self,
                    keyboard_report_state,
                    Modifier::RIGHT_ALT,
                    KeyCode::N7
                );
            }
            KeymapLayer::LowerAnsi => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::LEFT_BRACKET);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::PAGE_UP);
            }
            KeymapLayer::Num => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N8);
            }
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow1Col4 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::H);
            }
            KeymapLayer::QwertyAnsi | KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::J);
            }
            KeymapLayer::Lower => {
                autoshift_kc!(self, keyboard_report_state, KeyCode::N7);
            }
            KeymapLayer::LowerAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::SLASH);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::PRINT_SCREEN);
            }
            KeymapLayer::Num => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N7);
            }
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow1Col5 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::D);
            }
            KeymapLayer::QwertyAnsi | KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::H);
            }
            KeymapLayer::Lower => {
                with_modifier_kc!(
                    self,
                    keyboard_report_state,
                    Modifier::RIGHT_ALT,
                    KeyCode::DASH
                );
            }
            KeymapLayer::LowerAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::BACKSLASH);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::F12);
            }
            KeymapLayer::Num => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N6);
            }
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow2Col0 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        simple_push_pop_mod!(self, keyboard_report_state, Modifier::LEFT_SHIFT);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow2Col1 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::Z);
            }
            KeymapLayer::QwertyAnsi | KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::QUOTE);
            }
            KeymapLayer::Lower => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::SEMICOLON);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::INSERT);
            }
            KeymapLayer::LowerAnsi | KeymapLayer::Num => {}
            KeymapLayer::Settings => {
                if pressed {
                    keyboard_report_state.set_perm_layer(KeymapLayer::QwertyAnsi);
                }
            }
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow2Col2 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::V);
            }
            KeymapLayer::QwertyAnsi | KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::DOT);
            }
            KeymapLayer::Lower => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::QUOTE);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::END);
            }
            KeymapLayer::LowerAnsi | KeymapLayer::Num => {}
            KeymapLayer::Settings => {
                if pressed {
                    keyboard_report_state.set_perm_layer(KeymapLayer::DvorakAnsi);
                }
            }
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow2Col3 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::W);
            }
            KeymapLayer::QwertyAnsi | KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::COMMA);
            }
            KeymapLayer::Lower => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::LEFT_BRACKET);
            }
            KeymapLayer::Raise => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::PAGE_DOWN);
            }
            KeymapLayer::LowerAnsi | KeymapLayer::Num => {}
            KeymapLayer::Settings => {
                if pressed {
                    keyboard_report_state.set_perm_layer(KeymapLayer::DvorakSe);
                }
            }
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow2Col4 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::M);
            }
            KeymapLayer::QwertyAnsi | KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::M);
            }
            KeymapLayer::Lower => {
                with_modifier_kc!(
                    self,
                    keyboard_report_state,
                    Modifier::RIGHT_ALT,
                    KeyCode::NON_US_BACKSLASH
                );
            }
            KeymapLayer::LowerAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::PIPE);
            }
            KeymapLayer::Raise | KeymapLayer::Num => {}
            KeymapLayer::Settings => {
                if pressed {
                    keyboard_report_state.set_perm_layer(KeymapLayer::QwertyGaming);
                }
            }
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow2Col5 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::B);
            }
            KeymapLayer::QwertyAnsi | KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N);
            }
            KeymapLayer::Lower => {
                if pressed {
                    if keyboard_report_state.has_modifier(Modifier::ANY_SHIFT) {
                        keyboard_report_state.push_key(KeyCode::EQUALS);
                    } else {
                        keyboard_report_state.push_modifier(Modifier::LEFT_SHIFT);
                        keyboard_report_state.push_key(KeyCode::EQUALS);
                        keyboard_report_state.pop_key(KeyCode::EQUALS);
                        keyboard_report_state.push_key(KeyCode::EQUALS);
                        keyboard_report_state.jank.autoshifted = true;
                    }
                } else {
                    if keyboard_report_state.jank.autoshifted {
                        keyboard_report_state.pop_modifier(Modifier::LEFT_SHIFT);
                        keyboard_report_state.jank.autoshifted = false;
                    }
                    keyboard_report_state.pop_key(KeyCode::EQUALS);
                }
            }
            KeymapLayer::LowerAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::GRAVE);
            }
            KeymapLayer::Raise | KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow3Col0 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        simple_push_pop_mod!(self, keyboard_report_state, Modifier::RIGHT_CONTROL);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow3Col1 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakSe
            | KeymapLayer::DvorakAnsi
            | KeymapLayer::QwertyAnsi
            | KeymapLayer::QwertyGaming
            | KeymapLayer::Lower
            | KeymapLayer::LowerAnsi
            | KeymapLayer::Raise
            | KeymapLayer::Num => {
                if pressed {
                    keyboard_report_state.push_layer_with_fallback(KeymapLayer::Settings);
                }
            }
            KeymapLayer::Settings => {
                if !pressed {
                    keyboard_report_state.pop_layer(KeymapLayer::Settings);
                }
            }
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow3Col2 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        simple_push_pop_mod!(self, keyboard_report_state, Modifier::RIGHT_ALT);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow3Col3 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakSe
            | KeymapLayer::DvorakAnsi
            | KeymapLayer::QwertyAnsi
            | KeymapLayer::QwertyGaming
            | KeymapLayer::Lower
            | KeymapLayer::LowerAnsi
            | KeymapLayer::Num
            | KeymapLayer::Settings => {
                if pressed {
                    keyboard_report_state.push_layer_with_fallback(KeymapLayer::Raise);
                }
            }
            KeymapLayer::Raise => {
                if !pressed {
                    keyboard_report_state.pop_layer(KeymapLayer::Raise);
                }
            }
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow3Col4 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::I);
            }
            KeymapLayer::DvorakSe
            | KeymapLayer::DvorakAnsi
            | KeymapLayer::QwertyAnsi
            | KeymapLayer::Lower
            | KeymapLayer::LowerAnsi
            | KeymapLayer::Raise
            | KeymapLayer::Settings => {
                if pressed {
                    keyboard_report_state.push_layer_with_fallback(KeymapLayer::Num);
                }
            }
            KeymapLayer::Num => {
                if !pressed {
                    keyboard_report_state.pop_layer(KeymapLayer::Num);
                }
            }
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow3Col5 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        match keyboard_report_state.active_layer {
            KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe | KeymapLayer::QwertyAnsi => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::SPACE);
            }
            KeymapLayer::QwertyGaming => {
                simple_push_pop_kc!(self, keyboard_report_state, KeyCode::G);
            }
            KeymapLayer::Lower => {}
            KeymapLayer::LowerAnsi => {}
            KeymapLayer::Raise => {}
            KeymapLayer::Num => {}
            KeymapLayer::Settings => {}
        }
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow4Col1 {
    // Rotary encoder is here, no key
}

impl KeyboardButton for RightRow4Col2 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N2);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow4Col3 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N3);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow4Col4 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N4);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}

impl KeyboardButton for RightRow4Col5 {
    fn update_state(
        &mut self,
        pressed: bool,
        keyboard_report_state: &mut KeyboardReportState,
    ) -> bool {
        simple_push_pop_kc!(self, keyboard_report_state, KeyCode::N5);
        self.0.update_last_state(keyboard_report_state.next_seq(), keyboard_report_state.active_layer);
        true
    }
}
