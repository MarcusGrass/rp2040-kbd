mod mapping;

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
use crate::runtime::shared::cores_left::{push_layer_change, Producer};
use rp2040_kbd_lib::keycodes::{KeyCode, Modifier};
use rp2040_kbd_lib::matrix::{MatrixChange, MatrixUpdate};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum KeymapLayer {
    DvorakSe,
    DvorakAnsi,
    DvorakSeMac,
    QwertyGaming,
    Lower,
    LowerSeMac,
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
    generation: usize,
    inner_report: KeyboardReport,
    user_mods: Modifier,
    user_key_state: [u8; 6],
    outbound_reports: Queue<KeyboardReport, 16>,
    active_layer: KeymapLayer,
    last_perm_layer: Option<KeymapLayer>,
    jank: JankState,
}

#[expect(clippy::struct_excessive_bools)]
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
            generation: 0,
            inner_report: EMPTY_REPORT,
            user_mods: Modifier(0),
            user_key_state: [0; 6],
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
        }
    }

    pub fn increment_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    #[cfg(feature = "hiddev")]
    pub fn report(&self) -> Option<&KeyboardReport> {
        self.outbound_reports.peek()
    }

    #[cfg(feature = "hiddev")]
    pub fn accept(&mut self) {
        self.outbound_reports.pop_front();
    }

    fn restore_to_user_if_not_stale(&mut self, generation: usize) {
        if self.generation == generation.wrapping_add(1) {
            self.restore_to_user_state();
        }
    }

    fn restore_to_user_state(&mut self) {
        self.restore_to_user_mods();
        self.restore_to_user_keys();
    }

    fn restore_to_user_mods(&mut self) {
        if self.inner_report.modifier != self.user_mods.0 {
            self.inner_report.modifier = self.user_mods.0;
            self.report_current();
        }
    }

    fn restore_to_user_keys(&mut self) {
        if self.inner_report.keycodes != self.user_key_state {
            self.inner_report.keycodes = self.user_key_state;
            self.report_current();
        }
    }

    /// Restores state to user, then applies the new key
    fn push_key(&mut self, key_code: KeyCode) {
        self.push_key_raw(key_code);
        self.report_current();
    }

    fn push_key_raw(&mut self, key_code: KeyCode) {
        Self::push_key_to_arr(key_code, &mut self.user_key_state);
        Self::push_key_to_arr(key_code, &mut self.inner_report.keycodes);
    }

    fn push_key_to_arr(key_code: KeyCode, arr: &mut [u8; 6]) {
        for val in &mut *arr {
            if *val == 0 || *val == key_code.0 {
                *val = key_code.0;
                return;
            }
        }
        // Overflow, pop first, unlikely
        unsafe {
            #[cfg(feature = "serial")]
            {
                let _ = crate::runtime::shared::usb::acquire_usb()
                    .write_fmt(format_args!("Pre: {arr:?}\r\n"));
            }
            copy_within_unchecked(arr, 1, 5, 0);
            *arr.get_unchecked_mut(5) = key_code.0;
            #[cfg(feature = "serial")]
            {
                let _ = crate::runtime::shared::usb::acquire_usb()
                    .write_fmt(format_args!("Post: {arr:?}\r\n"));
            }
        }
    }

    fn report_current(&mut self) {
        self.outbound_reports
            .push_back(copy_report(&self.inner_report));
    }

    fn pop_key(&mut self, key_code: KeyCode) {
        if Self::pop_key_from_arr(key_code, &mut self.user_key_state) {
            self.inner_report.keycodes = self.user_key_state;
            self.report_current();
        }
    }

    fn temp_modify(
        &mut self,
        key_code: KeyCode,
        add_modifiers: &[Modifier],
        remove_modifiers: &[Modifier],
    ) {
        self.push_temp_modifiers(add_modifiers);
        self.temp_remove_modifiers(remove_modifiers);
        self.push_temp_key(key_code);
    }

    fn push_temp_modifiers(&mut self, modifier: &[Modifier]) {
        for m in modifier {
            if self.inner_report.modifier & m.0 == 0 {
                self.inner_report.modifier |= m.0;
                self.report_current();
            }
        }
    }

    fn temp_remove_modifiers(&mut self, modifier: &[Modifier]) {
        for m in modifier {
            if self.inner_report.modifier & m.0 != 0 {
                self.inner_report.modifier &= !m.0;
                self.report_current();
            }
        }
    }

    fn push_temp_key(&mut self, key_code: KeyCode) {
        Self::push_key_to_arr(key_code, &mut self.inner_report.keycodes);
        self.report_current();
    }

    fn pop_temp_key(&mut self, key_code: KeyCode) {
        if Self::pop_key_from_arr(key_code, &mut self.inner_report.keycodes) {
            self.report_current();
        }
    }

    pub fn pop_key_from_arr(key_code: KeyCode, arr: &mut [u8; 6]) -> bool {
        let mut at_ind = None;
        for (ind, val) in arr.iter().enumerate() {
            if *val == key_code.0 {
                at_ind = Some(ind);
                break;
            } else if *val == 0 {
                return false;
            }
        }
        if let Some(ind) = at_ind {
            unsafe {
                #[cfg(feature = "serial")]
                {
                    let _ = crate::runtime::shared::usb::acquire_usb()
                        .write_fmt(format_args!("Pre: {arr:?}\r\n"));
                }
                Self::pop_copy_back_arr(ind, arr);
                #[cfg(feature = "serial")]
                {
                    let _ = crate::runtime::shared::usb::acquire_usb()
                        .write_fmt(format_args!("Post: {arr:?}\r\n"));
                }
            }
            true
        } else {
            false
        }
    }

    unsafe fn pop_copy_back_arr(ind: usize, arr: &mut [u8; 6]) {
        *arr.get_unchecked_mut(ind) = 0;
        match ind {
            0 => {
                copy_within_unchecked(arr, 1, 5, 0);
                // Keys are shifted back by one, need to clear last or there'll be a duplication
                *arr.get_unchecked_mut(5) = 0;
            }
            1 => {
                copy_within_unchecked(arr, 2, 4, 1);
                *arr.get_unchecked_mut(5) = 0;
            }
            2 => {
                copy_within_unchecked(arr, 3, 3, 2);
                *arr.get_unchecked_mut(5) = 0;
            }
            3 => {
                copy_within_unchecked(arr, 4, 2, 3);
                *arr.get_unchecked_mut(5) = 0;
            }
            4 => {
                let old = *arr.get_unchecked(5);
                *arr.get_unchecked_mut(4) = old;
                *arr.get_unchecked_mut(5) = 0;
            }
            5 => {}
            _ => unreachable_unchecked(),
        }
    }

    #[inline]
    fn push_modifier(&mut self, modifier: Modifier) {
        if self.user_mods.0 & modifier.0 == 0 {
            self.user_mods.0 |= modifier.0;
        }
        if self.inner_report.modifier & modifier.0 == 0 {
            self.inner_report.modifier |= modifier.0;
            self.report_current();
        }
    }

    #[inline]
    fn pop_modifier(&mut self, modifier: Modifier) {
        if self.user_mods.0 & modifier.0 != 0 {
            self.user_mods.0 &= !modifier.0;
            if self.inner_report.modifier != self.user_mods.0 {
                self.inner_report.modifier = self.user_mods.0;
                self.report_current();
            }
        }
    }

    #[inline]
    fn has_user_modifier(&self, modifier: Modifier) -> bool {
        self.user_mods.0 & modifier.0 != 0
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
    }

    #[inline]
    fn pop_layer(&mut self, this: KeymapLayer) {
        if self.active_layer == this {
            if let Some(old) = self.last_perm_layer.take() {
                self.active_layer = old;
            }
        }
    }

    #[inline]
    fn set_perm_layer(&mut self, keymap_layer: KeymapLayer) {
        if keymap_layer != self.active_layer {
            self.active_layer = keymap_layer;
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

pub trait KeyboardButton {
    fn on_press(&mut self, _keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
    }

    fn on_release(
        &mut self,
        _last_press_state: LastPressState,
        _keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LastPressState {
    generation: usize,
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

    // Keys are individually very rarely pressed compared to the scan-loop latency,
    // this pretty small function not being inlined makes quite the difference.
    #[inline(never)]
    fn update_last_state(&mut self, current_state: &KeyboardReportState) {
        self.last_state = Some(LastPressState {
            generation: current_state.generation,
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

macro_rules! impl_check_update {
    ($entity: ty) => {
        impl $entity {
            #[inline(never)]
            pub fn check_update_state(
                &mut self,
                pressed: bool,
                keyboard_report_state: &mut KeyboardReportState,
                timer: Timer,
                producer: &Producer,
            ) -> bool {
                if self.0.jitter.try_submit(timer.get_counter(), pressed) {
                    if let Some(prev) = self.0.last_state.take() {
                        self.on_release(prev, keyboard_report_state, producer);
                    } else {
                        keyboard_report_state.restore_to_user_state();
                        self.on_press(keyboard_report_state, producer);
                        self.0.update_last_state(keyboard_report_state);
                        keyboard_report_state.increment_generation();
                    }
                    return true;
                }
                false
            }
        }
    };
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

impl_check_update!(LeftRow0Col0);
impl_check_update!(LeftRow0Col1);
impl_check_update!(LeftRow0Col2);
impl_check_update!(LeftRow0Col3);
impl_check_update!(LeftRow0Col4);
impl_check_update!(LeftRow0Col5);
impl_check_update!(LeftRow1Col0);
impl_check_update!(LeftRow1Col1);
impl_check_update!(LeftRow1Col2);
impl_check_update!(LeftRow1Col3);
impl_check_update!(LeftRow1Col4);
impl_check_update!(LeftRow1Col5);
impl_check_update!(LeftRow2Col0);
impl_check_update!(LeftRow2Col1);
impl_check_update!(LeftRow2Col2);
impl_check_update!(LeftRow2Col3);
impl_check_update!(LeftRow2Col4);
impl_check_update!(LeftRow2Col5);
impl_check_update!(LeftRow3Col0);
impl_check_update!(LeftRow3Col1);
impl_check_update!(LeftRow3Col2);
impl_check_update!(LeftRow3Col3);
impl_check_update!(LeftRow3Col4);
impl_check_update!(LeftRow3Col5);
impl_check_update!(LeftRow4Col1);
impl_check_update!(LeftRow4Col2);
impl_check_update!(LeftRow4Col3);
impl_check_update!(LeftRow4Col4);
impl_check_update!(LeftRow4Col5);

macro_rules! impl_read_pin_col {
    ($($structure: expr, $row: tt,)*, $col: tt) => {
        paste! {
            #[inline]
            pub fn [<read_col _ $col _pins>]($([< $structure:snake >]: &mut $structure,)* left_buttons: &mut LeftButtons, keyboard_report_state: &mut KeyboardReportState, timer: Timer, producer: &Producer) -> bool {
                // Safety: Make sure this is properly initialized and restored
                // at the end of this function, makes a noticeable difference in performance
                let col = unsafe {left_buttons.cols.$col.take().unwrap_unchecked()};
                let col = col.into_push_pull_output_in_state(PinState::Low);
                // Just pulling chibios defaults of 0.25 micros, could probably be 0
                crate::timer::wait_nanos(timer, 250);
                let bank = rp2040_hal::Sio::read_bank0();
                // Can immediately restore column, pins settle while we're reading the state that's
                // now in mem
                left_buttons.cols.$col = Some(col.into_pull_up_input());
                let mut any_change = false;
                $(
                    let state = bank & crate::keyboard::left::[<ROW $row>] == 0;
                    if [< $structure:snake >].0.is_pressed() != state && [< $structure:snake >].check_update_state(state, keyboard_report_state, timer, producer) {
                        any_change = true;
                    }

                )*
                // Wait for pins to settle
                while rp2040_hal::Sio::read_bank0() & crate::keyboard::left::ROW_MASK != crate::keyboard::left::ROW_MASK {}
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
    ($change: expr, $field: expr, $state: expr, $producer: expr) => {{
        if $change != $field.0.is_pressed() {
            if let Some(prev) = $field.0.last_state.take() {
                $field.on_release(prev, $state, $producer);
            } else {
                $state.restore_to_user_state();
                $field.on_press($state, $producer);
                $field.0.update_last_state($state);
                $state.increment_generation();
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
        producer: &Producer,
    ) -> bool {
        let col0_change = read_col_0_pins(
            &mut self.left_row0_col0,
            &mut self.left_row1_col0,
            &mut self.left_row2_col0,
            &mut self.left_row3_col0,
            left_buttons,
            keyboard_report_state,
            timer,
            producer,
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
            producer,
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
            producer,
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
            producer,
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
            producer,
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
            producer,
        );
        col0_change || col1_change || col2_change || col3_change || col4_change || col5_change
    }

    #[expect(clippy::too_many_lines)]
    pub fn update_right(
        &mut self,
        update: MatrixUpdate,
        keyboard_report_state: &mut KeyboardReportState,
        producer: &Producer,
    ) {
        match update.interpret_byte() {
            MatrixChange::EncoderUpdate(enc) => {
                rotate_layer(enc, keyboard_report_state, producer);
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
                    0 => handle_update_right!(
                        change,
                        self.right_row0_col0,
                        keyboard_report_state,
                        producer
                    ),
                    1 => handle_update_right!(
                        change,
                        self.right_row0_col1,
                        keyboard_report_state,
                        producer
                    ),
                    2 => handle_update_right!(
                        change,
                        self.right_row0_col2,
                        keyboard_report_state,
                        producer
                    ),
                    3 => handle_update_right!(
                        change,
                        self.right_row0_col3,
                        keyboard_report_state,
                        producer
                    ),
                    4 => handle_update_right!(
                        change,
                        self.right_row0_col4,
                        keyboard_report_state,
                        producer
                    ),
                    5 => handle_update_right!(
                        change,
                        self.right_row0_col5,
                        keyboard_report_state,
                        producer
                    ),
                    6 => handle_update_right!(
                        change,
                        self.right_row1_col0,
                        keyboard_report_state,
                        producer
                    ),
                    7 => handle_update_right!(
                        change,
                        self.right_row1_col1,
                        keyboard_report_state,
                        producer
                    ),
                    8 => handle_update_right!(
                        change,
                        self.right_row1_col2,
                        keyboard_report_state,
                        producer
                    ),
                    9 => handle_update_right!(
                        change,
                        self.right_row1_col3,
                        keyboard_report_state,
                        producer
                    ),
                    10 => handle_update_right!(
                        change,
                        self.right_row1_col4,
                        keyboard_report_state,
                        producer
                    ),
                    11 => handle_update_right!(
                        change,
                        self.right_row1_col5,
                        keyboard_report_state,
                        producer
                    ),
                    12 => handle_update_right!(
                        change,
                        self.right_row2_col0,
                        keyboard_report_state,
                        producer
                    ),
                    13 => handle_update_right!(
                        change,
                        self.right_row2_col1,
                        keyboard_report_state,
                        producer
                    ),
                    14 => handle_update_right!(
                        change,
                        self.right_row2_col2,
                        keyboard_report_state,
                        producer
                    ),
                    15 => handle_update_right!(
                        change,
                        self.right_row2_col3,
                        keyboard_report_state,
                        producer
                    ),
                    16 => handle_update_right!(
                        change,
                        self.right_row2_col4,
                        keyboard_report_state,
                        producer
                    ),
                    17 => handle_update_right!(
                        change,
                        self.right_row2_col5,
                        keyboard_report_state,
                        producer
                    ),
                    18 => handle_update_right!(
                        change,
                        self.right_row3_col0,
                        keyboard_report_state,
                        producer
                    ),
                    19 => handle_update_right!(
                        change,
                        self.right_row3_col1,
                        keyboard_report_state,
                        producer
                    ),
                    20 => handle_update_right!(
                        change,
                        self.right_row3_col2,
                        keyboard_report_state,
                        producer
                    ),
                    21 => handle_update_right!(
                        change,
                        self.right_row3_col3,
                        keyboard_report_state,
                        producer
                    ),
                    22 => handle_update_right!(
                        change,
                        self.right_row3_col4,
                        keyboard_report_state,
                        producer
                    ),
                    23 => handle_update_right!(
                        change,
                        self.right_row3_col5,
                        keyboard_report_state,
                        producer
                    ),
                    25 => handle_update_right!(
                        change,
                        self.right_row4_col1,
                        keyboard_report_state,
                        producer
                    ),
                    26 => handle_update_right!(
                        change,
                        self.right_row4_col2,
                        keyboard_report_state,
                        producer
                    ),
                    27 => handle_update_right!(
                        change,
                        self.right_row4_col3,
                        keyboard_report_state,
                        producer
                    ),
                    28 => handle_update_right!(
                        change,
                        self.right_row4_col4,
                        keyboard_report_state,
                        producer
                    ),
                    29 => handle_update_right!(
                        change,
                        self.right_row4_col5,
                        keyboard_report_state,
                        producer
                    ),
                    _ => {}
                }
            }
        }
    }
}

fn rotate_layer(
    clockwise: bool,
    keyboard_report_state: &mut KeyboardReportState,
    producer: &Producer,
) {
    match (
        keyboard_report_state.active_layer,
        keyboard_report_state.last_perm_layer,
    ) {
        (KeymapLayer::DvorakSe, _) => {
            if clockwise {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakAnsi);
                push_layer_change(producer, keyboard_report_state.active_layer);
            } else {
                keyboard_report_state.set_perm_layer(KeymapLayer::QwertyGaming);
                push_layer_change(producer, keyboard_report_state.active_layer);
            }
        }
        (KeymapLayer::DvorakAnsi, _) => {
            if clockwise {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakSeMac);
                push_layer_change(producer, keyboard_report_state.active_layer);
            } else {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakSe);
                push_layer_change(producer, keyboard_report_state.active_layer);
            }
        }
        (KeymapLayer::DvorakSeMac, _) => {
            if clockwise {
                keyboard_report_state.set_perm_layer(KeymapLayer::QwertyGaming);
                push_layer_change(producer, keyboard_report_state.active_layer);
            } else {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakAnsi);
                push_layer_change(producer, keyboard_report_state.active_layer);
            }
        }
        (KeymapLayer::QwertyGaming, _) => {
            if clockwise {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakSe);
                push_layer_change(producer, keyboard_report_state.active_layer);
            } else {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakAnsi);
                push_layer_change(producer, keyboard_report_state.active_layer);
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

#[macro_export]
macro_rules! temp_layer {
    ($layer: pat) => {
        (_, $layer)
    };
}

#[macro_export]
macro_rules! base_layer {
    ($layer: pat) => {
        (Some($layer), _) | (None, $layer)
    };
}
