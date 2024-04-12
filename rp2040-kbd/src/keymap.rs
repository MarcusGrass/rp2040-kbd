#[cfg(feature = "serial")]
use core::fmt::Write;
use core::hint::unreachable_unchecked;
use core::ptr;
use paste::paste;
use rp2040_hal::gpio::PinState;
use rp2040_hal::Timer;
use rp2040_kbd_lib::queue::Queue;
use usbd_hid::descriptor::KeyboardReport;

use crate::keyboard::debounce::PinDebouncer;
use crate::keyboard::left::LeftButtons;
use crate::runtime::shared::cores_left::push_reboot_and_halt;
use rp2040_kbd_lib::keycodes::{KeyCode, Modifier};
use rp2040_kbd_lib::matrix::{MatrixChange, MatrixUpdate};

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

const EMPTY_REPORT: KeyboardReport = KeyboardReport {
    modifier: 0,
    reserved: 0,
    leds: 0,
    keycodes: [0u8; 6],
};

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
    inner_report_has_change: bool,
    temp_mods: Option<Modifier>,
    outbound_reports: Queue<KeyboardReport, 16>,
    active_layer: KeymapLayer,
    last_perm_layer: Option<KeymapLayer>,
    jank: JankState,
    layer_change: Option<KeymapLayer>,
}

#[allow(clippy::struct_excessive_bools)]
struct JankState {
    pressing_double_quote: bool,
    pressing_single_quote: bool,
    pressing_left_bracket: bool,
    pressing_comma: bool,
    pressing_right_bracket: bool,
    pressing_dot: bool,
    pressing_reg_colon: bool,
    pressing_semicolon: bool,
}

impl KeyboardReportState {
    pub fn new() -> Self {
        Self {
            inner_report: EMPTY_REPORT,
            inner_report_has_change: true,
            temp_mods: None,
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
                pressing_reg_colon: false,
                pressing_semicolon: false,
            },
            layer_change: Some(KeymapLayer::DvorakSe),
        }
    }

    #[cfg(feature = "hiddev")]
    pub fn report(&self) -> Option<&KeyboardReport> {
        self.outbound_reports
            .peek()
            .or_else(|| self.inner_report_has_change.then_some(&self.inner_report))
    }

    #[cfg(feature = "hiddev")]
    pub fn accept(&mut self) {
        if self.outbound_reports.pop_front().is_none() {
            self.inner_report_has_change = false;
        }
    }

    pub fn layer_update(&mut self) -> Option<KeymapLayer> {
        self.layer_change.take()
    }

    pub fn push_key(&mut self, key_code: KeyCode) {
        self.pop_temp_modifiers();
        for val in &mut self.inner_report.keycodes {
            if *val == 0 || *val == key_code.0 {
                *val = key_code.0;
                self.inner_report_has_change = true;
                return;
            }
        }
        // Overflow, pop first, unlikely
        unsafe {
            copy_within_unchecked(&mut self.inner_report.keycodes, 1, 5, 0);
            *self.inner_report.keycodes.get_unchecked_mut(5) = key_code.0;
        }
        self.inner_report_has_change = true;
    }

    pub fn pop_key(&mut self, key_code: KeyCode) {
        self.pop_temp_modifiers();
        let mut at_ind = None;
        for (ind, val) in &mut self.inner_report.keycodes.iter().enumerate() {
            if *val == key_code.0 {
                at_ind = Some(ind);
                break;
            } else if *val == 0 {
                return;
            }
        }
        if let Some(ind) = at_ind {
            unsafe {
                self.pop_copy_back(ind);
            }
            self.outbound_reports
                .push_back(copy_report(&self.inner_report));
        }
    }

    unsafe fn pop_copy_back(&mut self, ind: usize) {
        *self.inner_report.keycodes.get_unchecked_mut(ind) = 0;
        match ind {
            0 => {
                copy_within_unchecked(&mut self.inner_report.keycodes, 1, 5, 0);
            }
            1 => {
                copy_within_unchecked(&mut self.inner_report.keycodes, 2, 4, 1);
            }
            2 => {
                copy_within_unchecked(&mut self.inner_report.keycodes, 3, 3, 2);
            }
            3 => {
                copy_within_unchecked(&mut self.inner_report.keycodes, 4, 2, 3);
            }
            4 => {
                let old = *self.inner_report.keycodes.get_unchecked(5);
                *self.inner_report.keycodes.get_unchecked_mut(4) = old;
            }
            5 => {}
            _ => unreachable_unchecked(),
        }
    }

    fn temp_modify(&mut self, key_code: KeyCode, add_mods: &[Modifier], pop_mods: &[Modifier]) {
        self.push_key(key_code);
        self.temp_mods = Some(Modifier(self.inner_report.modifier));
        for md in add_mods {
            self.inner_report.modifier |= md.0;
        }
        for md in pop_mods {
            self.inner_report.modifier &= !md.0;
        }
        self.inner_report_has_change = true;
        self.outbound_reports
            .push_back(copy_report(&self.inner_report));
    }

    #[inline]
    fn push_modifier(&mut self, modifier: Modifier) {
        self.pop_temp_modifiers();
        self.inner_report.modifier |= modifier.0;
        self.inner_report_has_change = true;
    }

    #[inline]
    fn pop_modifier(&mut self, modifier: Modifier) {
        self.pop_temp_modifiers();
        self.inner_report.modifier &= !modifier.0;
        self.inner_report_has_change = true;
    }

    #[inline]
    fn pop_temp_modifiers(&mut self) {
        if let Some(temp) = self.temp_mods.take() {
            self.inner_report.modifier = temp.0;
        }
    }

    #[inline]
    fn has_modifier(&self, modifier: Modifier) -> bool {
        self.temp_mods.map_or_else(
            || self.inner_report.modifier & modifier.0 != 0,
            |tm| tm.0 & modifier.0 != 0,
        )
    }

    /// Reset report on all layer switches
    #[inline]
    fn push_layer_with_fallback(&mut self, keymap_layer: KeymapLayer) {
        // If using a temp-layer, don't stack another temp-layer on top, pop the
        // non-temp first
        if let Some(old) = self.last_perm_layer.take() {
            self.active_layer = old;
        }
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
        if keymap_layer != self.active_layer {
            self.active_layer = keymap_layer;
            self.layer_change = Some(keymap_layer);
            self.last_perm_layer = None;
        }
    }
}

#[inline]
unsafe fn copy_within_unchecked(buf: &mut [u8; 6], src: usize, count: usize, dest: usize) {
    unsafe {
        let ptr = buf.as_mut_ptr();
        let src_ptr = ptr.add(src);
        let dest_ptr = ptr.add(dest);
        ptr::copy(src_ptr, dest_ptr, count);
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
    fn on_press(&mut self, _keyboard_report_state: &mut KeyboardReportState) {}

    fn on_release(
        &mut self,
        _last_press_state: LastPressState,
        _keyboard_report_state: &mut KeyboardReportState,
    ) {
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LastPressState {
    layer: KeymapLayer,
    last_perm_layer: Option<KeymapLayer>,
}

pub struct PinStructState {
    last_state: Option<LastPressState>,
    jitter: PinDebouncer,
}

impl PinStructState {
    #[inline]
    fn is_pressed(&self) -> bool {
        self.last_state.is_some()
    }

    #[inline(never)]
    fn update_last_state(&mut self, current_state: &mut KeyboardReportState) {
        self.last_state = Some(LastPressState {
            layer: current_state.active_layer,
            last_perm_layer: current_state.last_perm_layer,
        });
    }
}

impl PinStructState {
    pub const fn new() -> Self {
        Self {
            last_state: None,
            jitter: PinDebouncer::new(),
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
                                if let Some(prev) = self.0.last_state.take() {
                                    self.on_release(prev, keyboard_report_state);
                                } else {
                                    self.on_press(keyboard_report_state);
                                    self.0.update_last_state(keyboard_report_state);
                                }
                                return true;
                            }
                        }
                        false
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
            #[inline]
            pub fn [<read_col _ $col _pins>]($([< $structure:snake >]: &mut $structure,)* left_buttons: &mut LeftButtons, keyboard_report_state: &mut KeyboardReportState, timer: Timer) -> bool {
                // Safety: Make sure this is properly initialized and restored
                // at the end of this function, makes a noticeable difference in performance
                let col = unsafe {left_buttons.cols.$col.take().unwrap_unchecked()};
                let col = col.into_push_pull_output_in_state(PinState::Low);
                // Just pulling chibios defaults of 0.25 micros, could probably be 0
                crate::timer::wait_nanos(timer, 250);
                let mut any_change = false;
                $(
                    {
                        if [< $structure:snake >].check_update_state(left_buttons.row_pin_is_low(rp2040_kbd_lib::matrix::RowIndex::from_value($row)), keyboard_report_state, timer) {
                            any_change = true;
                        }
                    }

                )*
                left_buttons.cols.$col = Some(col.into_pull_up_input());
                $(
                    {
                        while left_buttons.row_pin_is_low(rp2040_kbd_lib::matrix::RowIndex::from_value($row)) {}
                    }
                )*
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

macro_rules! handle_update_right {
    ($change: expr, $field: expr, $state: expr) => {{
        if $change != $field.0.is_pressed() {
            if let Some(prev) = $field.0.last_state.take() {
                $field.on_release(prev, $state);
            } else {
                $field.on_press($state);
                $field.0.update_last_state($state);
            }
        }
    }};
}

impl KeyboardState {
    #[inline]
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

    #[allow(clippy::too_many_lines)]
    pub fn update_right(
        &mut self,
        update: MatrixUpdate,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match update.interpret_byte() {
            MatrixChange::EncoderUpdate(enc) => {
                rotate_layer(enc, keyboard_report_state);
            }
            MatrixChange::KeyUpdate(ind, change) => {
                #[cfg(feature = "serial")]
                {
                    let (row, col) = crate::keyboard::matrix_ind_to_row_col(ind.byte());
                    let _ = core::fmt::Write::write_fmt(
                        &mut crate::runtime::shared::usb::acquire_usb(),
                        format_args!("R: R{row}, C{col} -> {change}\r\n"),
                    );
                }
                match ind.byte() {
                    0 => handle_update_right!(change, self.right_row0_col0, keyboard_report_state),
                    1 => handle_update_right!(change, self.right_row0_col1, keyboard_report_state),
                    2 => handle_update_right!(change, self.right_row0_col2, keyboard_report_state),
                    3 => handle_update_right!(change, self.right_row0_col3, keyboard_report_state),
                    4 => handle_update_right!(change, self.right_row0_col4, keyboard_report_state),
                    5 => handle_update_right!(change, self.right_row0_col5, keyboard_report_state),
                    6 => handle_update_right!(change, self.right_row1_col0, keyboard_report_state),
                    7 => handle_update_right!(change, self.right_row1_col1, keyboard_report_state),
                    8 => handle_update_right!(change, self.right_row1_col2, keyboard_report_state),
                    9 => handle_update_right!(change, self.right_row1_col3, keyboard_report_state),
                    10 => handle_update_right!(change, self.right_row1_col4, keyboard_report_state),
                    11 => handle_update_right!(change, self.right_row1_col5, keyboard_report_state),
                    12 => handle_update_right!(change, self.right_row2_col0, keyboard_report_state),
                    13 => handle_update_right!(change, self.right_row2_col1, keyboard_report_state),
                    14 => handle_update_right!(change, self.right_row2_col2, keyboard_report_state),
                    15 => handle_update_right!(change, self.right_row2_col3, keyboard_report_state),
                    16 => handle_update_right!(change, self.right_row2_col4, keyboard_report_state),
                    17 => handle_update_right!(change, self.right_row2_col5, keyboard_report_state),
                    18 => handle_update_right!(change, self.right_row3_col0, keyboard_report_state),
                    19 => handle_update_right!(change, self.right_row3_col1, keyboard_report_state),
                    20 => handle_update_right!(change, self.right_row3_col2, keyboard_report_state),
                    21 => handle_update_right!(change, self.right_row3_col3, keyboard_report_state),
                    22 => handle_update_right!(change, self.right_row3_col4, keyboard_report_state),
                    23 => handle_update_right!(change, self.right_row3_col5, keyboard_report_state),
                    25 => handle_update_right!(change, self.right_row4_col1, keyboard_report_state),
                    26 => handle_update_right!(change, self.right_row4_col2, keyboard_report_state),
                    27 => handle_update_right!(change, self.right_row4_col3, keyboard_report_state),
                    28 => handle_update_right!(change, self.right_row4_col4, keyboard_report_state),
                    29 => handle_update_right!(change, self.right_row4_col5, keyboard_report_state),
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

macro_rules! temp_layer {
    ($layer: pat) => {
        (_, $layer)
    };
}

macro_rules! base_layer {
    ($layer: pat) => {
        (Some($layer), _) | (None, $layer)
    };
}

impl KeyboardButton for LeftRow0Col0 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        keyboard_report_state.push_key(KeyCode::TAB);
    }
    #[inline(never)]
    fn on_release(
        &mut self,
        _last_press_state: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        keyboard_report_state.pop_key(KeyCode::TAB);
    }
}

impl KeyboardButton for LeftRow0Col1 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F1);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::N1, &[Modifier::LEFT_SHIFT], &[]);
            }
            temp_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::Q);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::COMMA);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N1);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                if keyboard_report_state.has_modifier(Modifier::ANY_SHIFT) {
                    // Shifted, `SHIFT + 2` -> "
                    keyboard_report_state.jank.pressing_double_quote = true;
                    keyboard_report_state.push_key(KeyCode::N2);
                } else {
                    // Not shifted, \ -> '
                    keyboard_report_state.jank.pressing_single_quote = true;
                    keyboard_report_state.push_key(KeyCode::BACKSLASH);
                }
            }
            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        last_press_state: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (last_press_state.last_perm_layer, last_press_state.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F1);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::N1);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::COMMA);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                if keyboard_report_state.jank.pressing_double_quote {
                    keyboard_report_state.pop_key(KeyCode::N2);
                    keyboard_report_state.jank.pressing_double_quote = false;
                }
                if keyboard_report_state.jank.pressing_single_quote {
                    keyboard_report_state.jank.pressing_single_quote = false;
                    keyboard_report_state.pop_key(KeyCode::BACKSLASH);
                }
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N1);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::Q);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow0Col2 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Settings) => {}
            temp_layer!(KeymapLayer::Num) => {}
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F2);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::N2, &[Modifier::LEFT_SHIFT], &[]);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::N2, &[Modifier::RIGHT_ALT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::COMMA);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                if keyboard_report_state.has_modifier(Modifier::LEFT_SHIFT) {
                    // Need to remove shift for this key to go out, not putting it
                    // back after though for reasons that I don't remember and may be a bug
                    keyboard_report_state.temp_modify(
                        KeyCode::NON_US_BACKSLASH,
                        &[],
                        &[Modifier::LEFT_SHIFT],
                    );
                    keyboard_report_state.jank.pressing_left_bracket = true;
                } else {
                    keyboard_report_state.push_key(KeyCode::COMMA);
                    keyboard_report_state.jank.pressing_comma = true;
                }
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::W);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N2);
            }
            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F2);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::N2);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::N2);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::COMMA);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                if keyboard_report_state.jank.pressing_left_bracket {
                    keyboard_report_state.pop_key(KeyCode::NON_US_BACKSLASH);
                    keyboard_report_state.jank.pressing_left_bracket = false;
                }
                if keyboard_report_state.jank.pressing_comma {
                    keyboard_report_state.pop_key(KeyCode::COMMA);
                    keyboard_report_state.jank.pressing_comma = false;
                }
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::W);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N2);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow0Col3 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F3);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::N3, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::DOT);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                // Button is > or . with and without shift, respectively
                if keyboard_report_state.has_modifier(Modifier::LEFT_SHIFT) {
                    // Needs a shift, but that's already pressed
                    keyboard_report_state.push_key(KeyCode::NON_US_BACKSLASH);
                    keyboard_report_state.jank.pressing_right_bracket = true;
                } else {
                    keyboard_report_state.push_key(KeyCode::DOT);
                    keyboard_report_state.jank.pressing_dot = true;
                }
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::E);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N3);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F3);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::N3);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::DOT);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                if keyboard_report_state.jank.pressing_right_bracket {
                    keyboard_report_state.pop_key(KeyCode::NON_US_BACKSLASH);
                    keyboard_report_state.jank.pressing_right_bracket = false;
                }
                if keyboard_report_state.jank.pressing_dot {
                    keyboard_report_state.pop_key(KeyCode::DOT);
                    keyboard_report_state.jank.pressing_dot = false;
                }
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::E);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N3);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow0Col4 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F4);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::N4, &[Modifier::LEFT_SHIFT], &[]);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::N4, &[Modifier::RIGHT_ALT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::P);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::P);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::R);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N4);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F4);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::N4);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::N4);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::P);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::P);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::R);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N4);
            }

            _ => {}
        }
    }
}
impl KeyboardButton for LeftRow0Col5 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F4);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::N5, &[Modifier::LEFT_SHIFT], &[]);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::N5, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::Y);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::Y);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::T);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N5);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F4);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::N5);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::N5);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::Y);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::Y);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::T);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N5);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow1Col0 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        keyboard_report_state.push_key(KeyCode::ESCAPE);
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        keyboard_report_state.pop_key(KeyCode::ESCAPE);
    }
}

impl KeyboardButton for LeftRow1Col1 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.push_key(KeyCode::N1);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::LEFT_ARROW);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.push_key(KeyCode::KP_PLUS);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.push_key(KeyCode::DASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::A);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::A);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::A);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::T);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_key(KeyCode::N1);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::LEFT_ARROW);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::KP_PLUS);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::DASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::A);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::A);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::A);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::T);
            }

            _ => {}
        }
    }
}
impl KeyboardButton for LeftRow1Col2 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.push_key(KeyCode::N2);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::RIGHT_ARROW);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.push_key(KeyCode::EQUALS);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::N0, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::O);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::O);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::S);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::Q);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_key(KeyCode::N2);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::RIGHT_ARROW);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::EQUALS);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::N0);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::O);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::O);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::S);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::Q);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow1Col3 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.push_key(KeyCode::N3);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::UP_ARROW);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.push_key(KeyCode::LEFT_BRACKET);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::N8, &[Modifier::RIGHT_ALT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::E);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::E);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::D);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::W);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_key(KeyCode::N3);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::UP_ARROW);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::LEFT_BRACKET);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::N8);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::E);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::E);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::D);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::W);
            }

            _ => {}
        }
    }
}
impl KeyboardButton for LeftRow1Col4 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.push_key(KeyCode::N4);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::DOWN_ARROW);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.push_key(KeyCode::RIGHT_BRACKET);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::N9, &[Modifier::RIGHT_ALT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::U);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::U);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::F);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::E);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_key(KeyCode::N4);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::DOWN_ARROW);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::RIGHT_BRACKET);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::N9);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::U);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::U);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::F);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::E);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow1Col5 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.push_key(KeyCode::N5);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F11);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::SLASH, &[Modifier::LEFT_SHIFT], &[]);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::DASH, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::I);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::I);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::G);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::R);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_key(KeyCode::N5);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F11);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::SLASH);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::DASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::I);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::I);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::G);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::R);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow2Col0 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        keyboard_report_state.push_modifier(Modifier::LEFT_SHIFT);
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        keyboard_report_state.pop_modifier(Modifier::LEFT_SHIFT);
    }
}

impl KeyboardButton for LeftRow2Col1 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::SEMICOLON);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                if keyboard_report_state.has_modifier(Modifier::LEFT_SHIFT) {
                    // Needs a shift, but that's already pressed
                    keyboard_report_state.push_key(KeyCode::DOT);
                    keyboard_report_state.jank.pressing_reg_colon = true;
                } else {
                    keyboard_report_state.temp_modify(KeyCode::COMMA, &[Modifier::LEFT_SHIFT], &[]);
                    keyboard_report_state.jank.pressing_semicolon = true;
                }
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::Z);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::Y);
            }
            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::SEMICOLON);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                if keyboard_report_state.jank.pressing_reg_colon {
                    keyboard_report_state.pop_key(KeyCode::DOT);
                    keyboard_report_state.jank.pressing_reg_colon = false;
                }
                if keyboard_report_state.jank.pressing_semicolon {
                    keyboard_report_state.pop_key(KeyCode::COMMA);
                    keyboard_report_state.jank.pressing_semicolon = false;
                }
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::Z);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::Y);
            }
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow2Col2 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                // Copy
                keyboard_report_state.temp_modify(KeyCode::C, &[Modifier::LEFT_CONTROL], &[]);
            }
            temp_layer!(KeymapLayer::Lower) => {
                // Copy
                keyboard_report_state.temp_modify(KeyCode::C, &[Modifier::LEFT_CONTROL], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::Q);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::Q);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::X);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::A);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::C);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::Q);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::Q);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::X);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::A);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow2Col3 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::X, &[Modifier::LEFT_CONTROL], &[]);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::X, &[Modifier::LEFT_CONTROL], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::J);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::J);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::S);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::X);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::X);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::J);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::J);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::S);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow2Col4 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::V, &[Modifier::LEFT_CONTROL], &[]);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::V, &[Modifier::LEFT_CONTROL], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::K);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::K);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::V);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::D);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::V);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::V);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::K);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::K);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::D);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::V);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow2Col5 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::GRAVE, &[Modifier::LEFT_SHIFT], &[]);
            }
            temp_layer!(KeymapLayer::Lower) => {
                // ~ Tilde double-tap to get it out immediately
                keyboard_report_state.temp_modify(
                    KeyCode::RIGHT_BRACKET,
                    &[Modifier::RIGHT_ALT],
                    &[],
                );
                keyboard_report_state.pop_key(KeyCode::RIGHT_BRACKET);
                keyboard_report_state.temp_modify(
                    KeyCode::RIGHT_BRACKET,
                    &[Modifier::RIGHT_ALT],
                    &[],
                );
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::X);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::X);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::B);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::F);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::GRAVE);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::RIGHT_BRACKET);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::X);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::X);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::B);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::F);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow3Col0 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        keyboard_report_state.push_modifier(Modifier::LEFT_CONTROL);
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        keyboard_report_state.pop_modifier(Modifier::LEFT_CONTROL);
    }
}

impl KeyboardButton for LeftRow3Col1 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_modifier(Modifier::LEFT_GUI);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_modifier(Modifier::LEFT_GUI);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_modifier(Modifier::LEFT_GUI);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::Z);
            }
            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_modifier(Modifier::LEFT_GUI);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_modifier(Modifier::LEFT_GUI);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_modifier(Modifier::LEFT_GUI);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::Z);
            }
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow3Col2 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_modifier(Modifier::LEFT_ALT);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_modifier(Modifier::LEFT_ALT);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::X);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::X);
            }
            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_modifier(Modifier::LEFT_ALT);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_modifier(Modifier::LEFT_ALT);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::X);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::X);
            }
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow3Col3 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::DvorakSe) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            base_layer!(KeymapLayer::QwertyAnsi) => {}
            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::DvorakSe) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            base_layer!(KeymapLayer::QwertyAnsi) => {}
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow3Col4 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::LowerAnsi);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::LowerAnsi);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Lower);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::V);
            }
            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_layer(KeymapLayer::LowerAnsi);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_layer(KeymapLayer::Lower);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::V);
            }

            base_layer!(KeymapLayer::DvorakAnsi) => {}
            base_layer!(KeymapLayer::QwertyAnsi) => {}
            base_layer!(KeymapLayer::DvorakSe) => {}
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow3Col5 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::B);
            }
            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::B);
            }
            _ => {}
        }
    }
}

// Row 4 col 0 does not exist
impl KeyboardButton for LeftRow4Col1 {
    #[inline(never)]
    fn on_press(&mut self, _keyboard_report_state: &mut KeyboardReportState) {
        push_reboot_and_halt();
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        _keyboard_report_state: &mut KeyboardReportState,
    ) {
    }
}

impl KeyboardButton for LeftRow4Col2 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_modifier(Modifier::LEFT_GUI);
            }
            base_layer!(KeymapLayer::DvorakSe) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            base_layer!(KeymapLayer::QwertyAnsi) => {}
            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_modifier(Modifier::LEFT_GUI);
            }
            base_layer!(KeymapLayer::DvorakSe) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            base_layer!(KeymapLayer::QwertyAnsi) => {}
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow4Col3 {
    #[inline(never)]
    fn on_press(&mut self, _keyboard_report_state: &mut KeyboardReportState) {}

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        _keyboard_report_state: &mut KeyboardReportState,
    ) {
    }
}

impl KeyboardButton for LeftRow4Col4 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                // _
                keyboard_report_state.temp_modify(KeyCode::SLASH, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            base_layer!(KeymapLayer::QwertyAnsi) => {}
            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::SLASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            base_layer!(KeymapLayer::QwertyAnsi) => {}
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow4Col5 {
    #[inline(never)]
    fn on_press(&mut self, _keyboard_report_state: &mut KeyboardReportState) {}
    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        _keyboard_report_state: &mut KeyboardReportState,
    ) {
    }
}

// Right side, goes from right to left

impl KeyboardButton for RightRow0Col0 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        keyboard_report_state.push_key(KeyCode::BACKSPACE);
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        keyboard_report_state.pop_key(KeyCode::BACKSPACE);
    }
}

impl KeyboardButton for RightRow0Col1 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F10);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::BACKSLASH, &[Modifier::LEFT_SHIFT], &[]);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::BACKSLASH, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::L);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::L);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::P);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N0);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F10);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::N8);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::BACKSLASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::L);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::L);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::P);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N0);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow0Col2 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F9);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::N8, &[Modifier::LEFT_SHIFT], &[]);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::N9, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::R);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::R);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::O);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N9);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F9);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::N0);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::N9);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::R);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::R);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::O);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N9);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow0Col3 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F8);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::N9, &[Modifier::LEFT_SHIFT], &[]);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::N8, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::I);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N8);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F8);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::N9);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::N8);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::I);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N8);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow0Col4 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F7);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::N7, &[Modifier::LEFT_SHIFT], &[]);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::N6, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::G);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::G);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::U);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N7);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F7);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::N7);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::N6);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::G);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::G);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::U);
            }

            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N7);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow0Col5 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F6);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(KeyCode::N6, &[Modifier::LEFT_SHIFT], &[]);
            }
            temp_layer!(KeymapLayer::Lower) => {
                // Double-tap to get ^ on one press, not like I ever use circ for anything else
                if keyboard_report_state.has_modifier(Modifier::ANY_SHIFT) {
                    keyboard_report_state.push_key(KeyCode::RIGHT_BRACKET);
                    keyboard_report_state.pop_key(KeyCode::RIGHT_BRACKET);
                    keyboard_report_state.push_key(KeyCode::RIGHT_BRACKET);
                } else {
                    keyboard_report_state.temp_modify(
                        KeyCode::RIGHT_BRACKET,
                        &[Modifier::LEFT_SHIFT],
                        &[],
                    );
                    keyboard_report_state.pop_key(KeyCode::RIGHT_BRACKET);
                    keyboard_report_state.temp_modify(
                        KeyCode::RIGHT_BRACKET,
                        &[Modifier::LEFT_SHIFT],
                        &[],
                    );
                }
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::F);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::F);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::Y);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N6);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F6);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::N6);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::RIGHT_BRACKET);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::F);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::F);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::Y);
            }

            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N6);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow1Col0 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        keyboard_report_state.push_key(KeyCode::ENTER);
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        keyboard_report_state.pop_key(KeyCode::ENTER);
    }
}

impl KeyboardButton for RightRow1Col1 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.push_key(KeyCode::N0);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::KC_DELF);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.push_key(KeyCode::DASH);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.push_key(KeyCode::SLASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::S);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::S);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::SEMICOLON);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::SEMICOLON);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_key(KeyCode::N0);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::KC_DELF);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::DASH);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::SLASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::S);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::S);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::SEMICOLON);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::SEMICOLON);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow1Col2 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.push_key(KeyCode::N9);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::HOME);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(
                    KeyCode::RIGHT_BRACKET,
                    &[Modifier::RIGHT_ALT],
                    &[],
                );
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::N0, &[Modifier::RIGHT_ALT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::N);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::N);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::L);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::L);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_key(KeyCode::N9);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::HOME);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::RIGHT_BRACKET);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::N0);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::N);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::N);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::L);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::L);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow1Col3 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.push_key(KeyCode::N8);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::PAGE_UP);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.temp_modify(
                    KeyCode::LEFT_BRACKET,
                    &[Modifier::LEFT_SHIFT],
                    &[],
                );
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::N7, &[Modifier::RIGHT_ALT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::T);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::T);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::K);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::K);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_key(KeyCode::N8);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::PAGE_UP);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::LEFT_BRACKET);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::N7);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::T);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::T);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::K);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::K);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow1Col4 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.push_key(KeyCode::N7);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::PRINT_SCREEN);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.push_key(KeyCode::SLASH);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::N7, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::H);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::H);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::J);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::J);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::N7);
            }
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_key(KeyCode::N7);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::PRINT_SCREEN);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::SLASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::H);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::H);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::J);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::J);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow1Col5 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.push_key(KeyCode::N6);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F12);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.push_key(KeyCode::BACKSLASH);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(KeyCode::DASH, &[Modifier::RIGHT_ALT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::D);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::D);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::H);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::H);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_key(KeyCode::N6);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F12);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::BACKSLASH);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::DASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::D);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::D);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::H);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::H);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow2Col0 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        keyboard_report_state.push_modifier(Modifier::LEFT_SHIFT);
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        keyboard_report_state.pop_modifier(Modifier::LEFT_SHIFT);
    }
}

impl KeyboardButton for RightRow2Col1 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Settings) => {
                keyboard_report_state.set_perm_layer(KeymapLayer::QwertyAnsi);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::INSERT);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.push_key(KeyCode::SEMICOLON);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::Z);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::Z);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::QUOTE);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::QUOTE);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::INSERT);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::SEMICOLON);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::Z);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::Z);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::QUOTE);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::QUOTE);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow2Col2 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Settings) => {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakAnsi);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::END);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.push_key(KeyCode::QUOTE);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::V);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::V);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::DOT);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::DOT);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::END);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::QUOTE);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::V);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::V);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::DOT);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::DOT);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow2Col3 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Settings) => {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakSe);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::PAGE_DOWN);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.push_key(KeyCode::LEFT_BRACKET);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::W);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::W);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::COMMA);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::COMMA);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::PAGE_DOWN);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::LEFT_BRACKET);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::W);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::W);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::COMMA);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::COMMA);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow2Col4 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Settings) => {
                keyboard_report_state.set_perm_layer(KeymapLayer::QwertyGaming);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.push_key(KeyCode::PIPE);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.temp_modify(
                    KeyCode::NON_US_BACKSLASH,
                    &[Modifier::RIGHT_ALT],
                    &[],
                );
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::M);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::M);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::M);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::M);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::PIPE);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::NON_US_BACKSLASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::M);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::M);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::M);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::M);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow2Col5 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.push_key(KeyCode::GRAVE);
            }
            temp_layer!(KeymapLayer::Lower) => {
                if keyboard_report_state.has_modifier(Modifier::ANY_SHIFT) {
                    keyboard_report_state.push_key(KeyCode::EQUALS);
                } else {
                    keyboard_report_state.temp_modify(
                        KeyCode::EQUALS,
                        &[Modifier::LEFT_SHIFT],
                        &[],
                    );
                    keyboard_report_state.pop_key(KeyCode::EQUALS);
                    keyboard_report_state.temp_modify(
                        KeyCode::EQUALS,
                        &[Modifier::LEFT_SHIFT],
                        &[],
                    );
                }
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::B);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::B);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::N);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::GRAVE);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_key(KeyCode::EQUALS);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::B);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::B);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::N);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow3Col0 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        keyboard_report_state.push_modifier(Modifier::LEFT_CONTROL);
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        keyboard_report_state.pop_modifier(Modifier::LEFT_CONTROL);
    }
}

impl KeyboardButton for RightRow3Col1 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Settings) => {}
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Settings);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Settings);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Settings);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Settings);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Settings) => {
                keyboard_report_state.pop_layer(KeymapLayer::Settings);
            }
            base_layer!(KeymapLayer::DvorakSe) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            base_layer!(KeymapLayer::QwertyAnsi) => {}
            base_layer!(KeymapLayer::QwertyGaming) => {}

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow3Col2 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        keyboard_report_state.push_modifier(Modifier::RIGHT_ALT);
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        keyboard_report_state.pop_modifier(Modifier::RIGHT_ALT);
    }
}

impl KeyboardButton for RightRow3Col3 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {}
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Raise);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Raise);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Raise);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Raise);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_layer(KeymapLayer::Raise);
            }
            base_layer!(KeymapLayer::DvorakSe) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            base_layer!(KeymapLayer::QwertyAnsi) => {}
            base_layer!(KeymapLayer::QwertyGaming) => {}

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow3Col4 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Num) => {}
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::I);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Num);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Num);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Num);
            }

            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_layer(KeymapLayer::Num);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::I);
            }
            base_layer!(KeymapLayer::DvorakSe) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            base_layer!(KeymapLayer::QwertyAnsi) => {}

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow3Col5 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.push_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::G);
            }
            _ => {}
        }
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.pop_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::QwertyAnsi) => {
                keyboard_report_state.pop_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::G);
            }
            _ => {}
        }
    }
}

impl KeyboardButton for RightRow4Col1 {
    #[inline(never)]
    fn on_press(&mut self, _keyboard_report_state: &mut KeyboardReportState) {}

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        _keyboard_report_state: &mut KeyboardReportState,
    ) {
    }
    // Rotary encoder is here, no key
}

impl KeyboardButton for RightRow4Col2 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        keyboard_report_state.push_key(KeyCode::N2);
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        keyboard_report_state.pop_key(KeyCode::N2);
    }
}

impl KeyboardButton for RightRow4Col3 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        keyboard_report_state.push_key(KeyCode::N3);
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        keyboard_report_state.pop_key(KeyCode::N3);
    }
}

impl KeyboardButton for RightRow4Col4 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        keyboard_report_state.push_key(KeyCode::N4);
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        keyboard_report_state.pop_key(KeyCode::N4);
    }
}

impl KeyboardButton for RightRow4Col5 {
    #[inline(never)]
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState) {
        keyboard_report_state.push_key(KeyCode::N5);
    }

    #[inline(never)]
    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
    ) {
        keyboard_report_state.pop_key(KeyCode::N5);
    }
}
