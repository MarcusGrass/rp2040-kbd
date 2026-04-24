use super::{
    KeyboardButton, KeyboardReportState, KeymapLayer, LastPressState, LeftRow0Col0, LeftRow0Col1,
    LeftRow0Col2, LeftRow0Col3, LeftRow0Col4, LeftRow0Col5, LeftRow1Col0, LeftRow1Col1,
    LeftRow1Col2, LeftRow1Col3, LeftRow1Col4, LeftRow1Col5, LeftRow2Col0, LeftRow2Col1,
    LeftRow2Col2, LeftRow2Col3, LeftRow2Col4, LeftRow2Col5, LeftRow3Col0, LeftRow3Col1,
    LeftRow3Col2, LeftRow3Col3, LeftRow3Col4, LeftRow3Col5, LeftRow4Col1, LeftRow4Col2,
    LeftRow4Col3, LeftRow4Col4, LeftRow4Col5, RightRow0Col0, RightRow0Col1, RightRow0Col2,
    RightRow0Col3, RightRow0Col4, RightRow0Col5, RightRow1Col0, RightRow1Col1, RightRow1Col2,
    RightRow1Col3, RightRow1Col4, RightRow1Col5, RightRow2Col0, RightRow2Col1, RightRow2Col2,
    RightRow2Col3, RightRow2Col4, RightRow2Col5, RightRow3Col0, RightRow3Col1, RightRow3Col2,
    RightRow3Col3, RightRow3Col4, RightRow3Col5, RightRow4Col1, RightRow4Col2, RightRow4Col3,
    RightRow4Col4, RightRow4Col5,
};
use crate::runtime::shared::cores_left::{push_layer_change, push_reboot_and_halt, Producer};
use crate::{base_layer, temp_layer};
use rp2040_kbd_lib::keycodes::{KeyCode, Modifier};

impl KeyboardButton for LeftRow0Col0 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        keyboard_report_state.push_key(KeyCode::TAB);
    }
    fn on_release(
        &mut self,
        _last_press_state: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        keyboard_report_state.pop_key(KeyCode::TAB);
    }
}

impl KeyboardButton for LeftRow0Col1 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F1);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerAnsi | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::N1, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::COMMA);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N1);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                if keyboard_report_state.has_user_modifier(Modifier::ANY_SHIFT) {
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

    fn on_release(
        &mut self,
        last_press_state: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (last_press_state.last_perm_layer, last_press_state.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F1);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerAnsi | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(last_press_state.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::COMMA);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
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
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow0Col2 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            // Not particularly different, but it's safest to stick with L_ALT for macos
            temp_layer!(KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::N2, &[Modifier::LEFT_ALT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::COMMA);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                if keyboard_report_state.has_user_modifier(Modifier::LEFT_SHIFT) {
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
            base_layer!(KeymapLayer::DvorakSeMac) => {
                if keyboard_report_state.has_user_modifier(Modifier::LEFT_SHIFT) {
                    // Need to remove shift for this key to go out, not putting it
                    // back after though for reasons that I don't remember and may be a bug
                    keyboard_report_state.temp_modify(KeyCode::GRAVE, &[], &[Modifier::LEFT_SHIFT]);
                    keyboard_report_state.jank.pressing_left_bracket = true;
                } else {
                    keyboard_report_state.push_key(KeyCode::COMMA);
                    keyboard_report_state.jank.pressing_comma = true;
                }
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N2);
            }
            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F2);
            }
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::COMMA);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                if keyboard_report_state.jank.pressing_left_bracket {
                    // These are on the same button and interfere with each other
                    if !keyboard_report_state.jank.pressing_right_bracket {
                        keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
                    }
                    keyboard_report_state.jank.pressing_left_bracket = false;
                }
                if keyboard_report_state.jank.pressing_comma {
                    keyboard_report_state.pop_key(KeyCode::COMMA);
                    keyboard_report_state.jank.pressing_comma = false;
                }
            }
            base_layer!(KeymapLayer::DvorakSeMac) => {
                if keyboard_report_state.jank.pressing_left_bracket {
                    // These are on the same button and interfere with each other
                    if !keyboard_report_state.jank.pressing_right_bracket {
                        keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
                    }
                    keyboard_report_state.jank.pressing_left_bracket = false;
                }
                if keyboard_report_state.jank.pressing_comma {
                    keyboard_report_state.pop_key(KeyCode::COMMA);
                    keyboard_report_state.jank.pressing_comma = false;
                }
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N2);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow0Col3 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F3);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerAnsi | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::N3, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::DOT);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                // Button is > or . with and without shift, respectively
                if keyboard_report_state.has_user_modifier(Modifier::LEFT_SHIFT) {
                    keyboard_report_state.push_key(KeyCode::NON_US_BACKSLASH);
                    keyboard_report_state.jank.pressing_right_bracket = true;
                } else {
                    keyboard_report_state.push_key(KeyCode::DOT);
                    keyboard_report_state.jank.pressing_dot = true;
                }
            }
            base_layer!(KeymapLayer::DvorakSeMac) => {
                // Button is > or . with and without shift, respectively
                if keyboard_report_state.has_user_modifier(Modifier::LEFT_SHIFT) {
                    keyboard_report_state.push_key(KeyCode::GRAVE);
                    keyboard_report_state.jank.pressing_right_bracket = true;
                } else {
                    keyboard_report_state.push_key(KeyCode::DOT);
                    keyboard_report_state.jank.pressing_dot = true;
                }
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N3);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F3);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerAnsi | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
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
            base_layer!(KeymapLayer::DvorakSeMac) => {
                if keyboard_report_state.jank.pressing_right_bracket {
                    keyboard_report_state.pop_key(KeyCode::GRAVE);
                    keyboard_report_state.jank.pressing_right_bracket = false;
                }
                if keyboard_report_state.jank.pressing_dot {
                    keyboard_report_state.pop_key(KeyCode::DOT);
                    keyboard_report_state.jank.pressing_dot = false;
                }
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N3);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow0Col4 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::N4, &[Modifier::RIGHT_ALT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::P);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::P);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N4);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F4);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::P);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::P);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N4);
            }

            _ => {}
        }
    }
}
impl KeyboardButton for LeftRow0Col5 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::N5, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::Y);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::Y);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N5);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F4);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::Y);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::Y);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N5);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow1Col0 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        keyboard_report_state.push_key(KeyCode::ESCAPE);
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        keyboard_report_state.pop_key(KeyCode::ESCAPE);
    }
}

impl KeyboardButton for LeftRow1Col1 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.push_key(KeyCode::DASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::A);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::A);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::T);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.pop_key(KeyCode::DASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::A);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
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
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::N0, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::O);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::O);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::Q);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::O);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::O);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::Q);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow1Col3 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::N8, &[Modifier::RIGHT_ALT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::E);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::E);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::W);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::E);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::E);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::W);
            }

            _ => {}
        }
    }
}
impl KeyboardButton for LeftRow1Col4 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::N9, &[Modifier::RIGHT_ALT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::U);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::U);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::E);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::U);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::U);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::E);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow1Col5 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::DASH, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::I);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::I);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::R);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_key(KeyCode::N5);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F11);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::I);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::I);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::R);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow2Col0 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        keyboard_report_state.push_modifier(Modifier::LEFT_SHIFT);
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        keyboard_report_state.pop_modifier(Modifier::LEFT_SHIFT);
    }
}

impl KeyboardButton for LeftRow2Col1 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::SEMICOLON);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                if keyboard_report_state.has_user_modifier(Modifier::LEFT_SHIFT) {
                    // Needs a shift, but that's already pressed
                    keyboard_report_state.temp_modify(KeyCode::DOT, &[Modifier::LEFT_SHIFT], &[]);
                    keyboard_report_state.jank.pressing_reg_colon = true;
                } else {
                    keyboard_report_state.temp_modify(KeyCode::COMMA, &[Modifier::LEFT_SHIFT], &[]);
                    keyboard_report_state.jank.pressing_semicolon = true;
                }
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::Y);
            }
            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::SEMICOLON);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
                if keyboard_report_state.jank.pressing_reg_colon {
                    keyboard_report_state.jank.pressing_reg_colon = false;
                }
                if keyboard_report_state.jank.pressing_semicolon {
                    keyboard_report_state.jank.pressing_semicolon = false;
                }
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::Y);
            }
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow2Col2 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                // Copy
                keyboard_report_state.temp_modify(KeyCode::C, &[Modifier::LEFT_CONTROL], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::Q);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::Q);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::A);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::Q);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::Q);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::A);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow2Col3 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::X, &[Modifier::LEFT_CONTROL], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::J);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::J);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::S);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::J);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::J);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::S);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow2Col4 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::V, &[Modifier::LEFT_CONTROL], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::K);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::K);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::D);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::K);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::K);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::D);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow2Col5 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
                keyboard_report_state.pop_temp_key(KeyCode::RIGHT_BRACKET);
                keyboard_report_state.temp_modify(
                    KeyCode::RIGHT_BRACKET,
                    &[Modifier::RIGHT_ALT],
                    &[],
                );
            }
            temp_layer!(KeymapLayer::LowerSeMac) => {
                // ~ Tilde double-tap to get it out immediately
                keyboard_report_state.temp_modify(
                    KeyCode::RIGHT_BRACKET,
                    &[Modifier::LEFT_ALT],
                    &[],
                );
                keyboard_report_state.pop_temp_key(KeyCode::RIGHT_BRACKET);
                keyboard_report_state.temp_modify(KeyCode::SPACE, &[], &[Modifier::LEFT_ALT]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::X);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::X);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::F);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::X);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::X);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::F);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow3Col0 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        keyboard_report_state.push_modifier(Modifier::LEFT_CONTROL);
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        keyboard_report_state.pop_modifier(Modifier::LEFT_CONTROL);
    }
}

impl KeyboardButton for LeftRow3Col1 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_modifier(Modifier::LEFT_GUI);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_modifier(Modifier::LEFT_GUI);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::Z);
            }
            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_modifier(Modifier::LEFT_GUI);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
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
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_modifier(Modifier::LEFT_ALT);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_modifier(Modifier::LEFT_ALT);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::X);
            }
            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_modifier(Modifier::LEFT_ALT);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_modifier(Modifier::LEFT_ALT);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::X);
            }
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow3Col3 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow3Col4 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::LowerAnsi);
                push_layer_change(producer, KeymapLayer::DvorakAnsi);
            }
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Lower);
                push_layer_change(producer, KeymapLayer::DvorakSe);
            }
            base_layer!(KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::LowerSeMac);
                push_layer_change(producer, KeymapLayer::DvorakSeMac);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::V);
            }
            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_layer(KeymapLayer::LowerAnsi);
                push_layer_change(producer, keyboard_report_state.active_layer);
            }
            temp_layer!(KeymapLayer::Lower) => {
                keyboard_report_state.pop_layer(KeymapLayer::Lower);
                push_layer_change(producer, keyboard_report_state.active_layer);
            }
            temp_layer!(KeymapLayer::LowerSeMac) => {
                keyboard_report_state.pop_layer(KeymapLayer::LowerSeMac);
                push_layer_change(producer, keyboard_report_state.active_layer);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::V);
            }

            base_layer!(KeymapLayer::DvorakAnsi) => {}
            base_layer!(KeymapLayer::DvorakSe) => {}
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow3Col5 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::B);
            }
            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
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
    fn on_press(&mut self, _keyboard_report_state: &mut KeyboardReportState, producer: &Producer) {
        push_reboot_and_halt(producer);
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        _keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
    }
}

impl KeyboardButton for LeftRow4Col2 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_modifier(Modifier::LEFT_GUI);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_modifier(Modifier::LEFT_GUI);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow4Col3 {
    fn on_press(&mut self, _keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        _keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
    }
}

impl KeyboardButton for LeftRow4Col4 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                // _
                keyboard_report_state.temp_modify(KeyCode::SLASH, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            _ => {}
        }
    }
}

impl KeyboardButton for LeftRow4Col5 {
    fn on_press(&mut self, _keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
    }
    fn on_release(
        &mut self,
        _prev: LastPressState,
        _keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
    }
}

// Right side, goes from right to left

impl KeyboardButton for RightRow0Col0 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        keyboard_report_state.push_key(KeyCode::BACKSPACE);
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        keyboard_report_state.pop_key(KeyCode::BACKSPACE);
    }
}

impl KeyboardButton for RightRow0Col1 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::F10);
            }
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::BACKSLASH, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(
                KeymapLayer::DvorakAnsi | KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac
            ) => {
                keyboard_report_state.push_key(KeyCode::L);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N0);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F10);
            }
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::L);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::L);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N0);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow0Col2 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::N9, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::R);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::R);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N9);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F9);
            }
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::R);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::R);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N9);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow0Col3 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::N8, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N8);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F8);
            }
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::C);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N8);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow0Col4 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::N6, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::G);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::G);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N7);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F7);
            }
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::G);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::G);
            }

            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N7);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow0Col5 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
                keyboard_report_state.temp_modify(
                    KeyCode::RIGHT_BRACKET,
                    &[Modifier::LEFT_SHIFT],
                    &[],
                );
                keyboard_report_state.push_temp_key(KeyCode::RIGHT_BRACKET);
                keyboard_report_state.temp_modify(KeyCode::SPACE, &[], &[Modifier::LEFT_SHIFT]);
            }
            temp_layer!(KeymapLayer::LowerSeMac) => {
                // macos circ (^)
                keyboard_report_state.temp_modify(
                    KeyCode::RIGHT_BRACKET,
                    &[Modifier::LEFT_SHIFT],
                    &[],
                );
                keyboard_report_state.push_temp_key(KeyCode::RIGHT_BRACKET);
                keyboard_report_state.temp_modify(KeyCode::SPACE, &[], &[Modifier::LEFT_SHIFT]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::F);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::F);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N6);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::F6);
            }
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::F);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::F);
            }

            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N6);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow1Col0 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        keyboard_report_state.push_key(KeyCode::ENTER);
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        keyboard_report_state.pop_key(KeyCode::ENTER);
    }
}

impl KeyboardButton for RightRow1Col1 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.push_key(KeyCode::SLASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::S);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::S);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::SEMICOLON);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.pop_key(KeyCode::SLASH);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::S);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::S);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::SEMICOLON);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow1Col2 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(
                    KeyCode::N9,
                    &[Modifier::LEFT_SHIFT, Modifier::LEFT_ALT],
                    &[],
                );
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::N);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::N);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::L);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_key(KeyCode::N9);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::HOME);
            }
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::N);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::N);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::L);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow1Col3 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(
                    KeyCode::N8,
                    &[Modifier::LEFT_SHIFT, Modifier::LEFT_ALT],
                    &[],
                );
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::T);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::T);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::K);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_key(KeyCode::N8);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::PAGE_UP);
            }
            temp_layer!(KeymapLayer::LowerAnsi | KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::T);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::T);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::K);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow1Col4 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::N7, &[Modifier::LEFT_SHIFT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::H);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::H);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::J);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
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
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::H);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::J);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow1Col5 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
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
            temp_layer!(KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(
                    KeyCode::N7,
                    &[Modifier::LEFT_SHIFT, Modifier::LEFT_ALT],
                    &[],
                );
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::D);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::D);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::H);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
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
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::D);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::D);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::H);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow2Col0 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        keyboard_report_state.push_modifier(Modifier::LEFT_SHIFT);
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        keyboard_report_state.pop_modifier(Modifier::LEFT_SHIFT);
    }
}

impl KeyboardButton for RightRow2Col1 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::INSERT);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.push_key(KeyCode::SEMICOLON);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::Z);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::Z);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::QUOTE);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::INSERT);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.pop_key(KeyCode::SEMICOLON);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::Z);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::Z);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::QUOTE);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow2Col2 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Settings) => {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakAnsi);
                push_layer_change(producer, keyboard_report_state.active_layer);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::END);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.push_key(KeyCode::QUOTE);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::V);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::V);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::DOT);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::END);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.pop_key(KeyCode::QUOTE);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::V);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::V);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::DOT);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow2Col3 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Settings) => {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakSe);
                push_layer_change(producer, keyboard_report_state.active_layer);
            }
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.push_key(KeyCode::PAGE_DOWN);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.push_key(KeyCode::LEFT_BRACKET);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::W);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::W);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::COMMA);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_key(KeyCode::PAGE_DOWN);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.pop_key(KeyCode::LEFT_BRACKET);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::W);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::W);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::COMMA);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow2Col4 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Settings) => {
                keyboard_report_state.set_perm_layer(KeymapLayer::QwertyGaming);
                push_layer_change(producer, keyboard_report_state.active_layer);
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
            temp_layer!(KeymapLayer::LowerSeMac) => {
                keyboard_report_state.temp_modify(KeyCode::N7, &[Modifier::LEFT_ALT], &[]);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::M);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::M);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::M);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::PIPE);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::M);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
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
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Settings) => {
                keyboard_report_state.set_perm_layer(KeymapLayer::DvorakSeMac);
                push_layer_change(producer, keyboard_report_state.active_layer);
            }
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.push_key(KeyCode::GRAVE);
            }
            temp_layer!(KeymapLayer::Lower) => {
                if keyboard_report_state.has_user_modifier(Modifier::ANY_SHIFT) {
                    keyboard_report_state.push_key(KeyCode::EQUALS);
                } else {
                    keyboard_report_state.temp_modify(
                        KeyCode::EQUALS,
                        &[Modifier::LEFT_SHIFT],
                        &[],
                    );
                    keyboard_report_state.pop_temp_key(KeyCode::EQUALS);
                    keyboard_report_state.push_temp_key(KeyCode::EQUALS);
                }
            }
            temp_layer!(KeymapLayer::LowerSeMac) => {
                // Todo: Check if correct (on linux as well for theh above)
                keyboard_report_state.temp_modify(KeyCode::EQUALS, &[Modifier::LEFT_SHIFT], &[]);
                keyboard_report_state.pop_temp_key(KeyCode::EQUALS);
                keyboard_report_state.push_temp_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::B);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::B);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::N);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::LowerAnsi) => {
                keyboard_report_state.pop_key(KeyCode::GRAVE);
            }
            temp_layer!(KeymapLayer::Lower | KeymapLayer::LowerSeMac) => {
                keyboard_report_state.restore_to_user_if_not_stale(prev.generation);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::B);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.pop_key(KeyCode::B);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.pop_key(KeyCode::N);
            }

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow3Col0 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        keyboard_report_state.push_modifier(Modifier::LEFT_CONTROL);
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        keyboard_report_state.pop_modifier(Modifier::LEFT_CONTROL);
    }
}

impl KeyboardButton for RightRow3Col1 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Settings) => {}
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Settings);
                push_layer_change(producer, KeymapLayer::DvorakSe);
            }
            base_layer!(KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Settings);
                push_layer_change(producer, KeymapLayer::DvorakSeMac);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Settings);
                push_layer_change(producer, KeymapLayer::DvorakAnsi);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Settings);
                push_layer_change(producer, KeymapLayer::QwertyGaming);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Settings) => {
                keyboard_report_state.pop_layer(KeymapLayer::Settings);
                push_layer_change(producer, keyboard_report_state.active_layer);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            base_layer!(KeymapLayer::QwertyGaming) => {}

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow3Col2 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        keyboard_report_state.push_modifier(Modifier::RIGHT_ALT);
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        keyboard_report_state.pop_modifier(Modifier::RIGHT_ALT);
    }
}

impl KeyboardButton for RightRow3Col3 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            temp_layer!(KeymapLayer::Raise) => {}
            base_layer!(KeymapLayer::DvorakSe) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Raise);
                push_layer_change(producer, KeymapLayer::DvorakSe);
            }
            base_layer!(KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Raise);
                push_layer_change(producer, KeymapLayer::DvorakSeMac);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Raise);
                push_layer_change(producer, KeymapLayer::DvorakAnsi);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Raise);
                push_layer_change(producer, KeymapLayer::QwertyGaming);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Raise) => {
                keyboard_report_state.pop_layer(KeymapLayer::Raise);
                push_layer_change(producer, keyboard_report_state.active_layer);
            }
            base_layer!(KeymapLayer::DvorakSe) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            base_layer!(KeymapLayer::QwertyGaming) => {}

            _ => {}
        }
    }
}

impl KeyboardButton for RightRow3Col4 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, producer: &Producer) {
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
                push_layer_change(producer, KeymapLayer::DvorakSe);
            }
            base_layer!(KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Num);
                push_layer_change(producer, KeymapLayer::DvorakSeMac);
            }
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_layer_with_fallback(KeymapLayer::Num);
                push_layer_change(producer, KeymapLayer::DvorakAnsi);
            }

            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            temp_layer!(KeymapLayer::Num) => {
                keyboard_report_state.pop_layer(KeymapLayer::Num);
                push_layer_change(producer, keyboard_report_state.active_layer);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::I);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {}
            base_layer!(KeymapLayer::DvorakAnsi) => {}
            _ => {}
        }
    }
}

impl KeyboardButton for RightRow3Col5 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        match (
            keyboard_report_state.last_perm_layer,
            keyboard_report_state.active_layer,
        ) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.push_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
                keyboard_report_state.push_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::QwertyGaming) => {
                keyboard_report_state.push_key(KeyCode::G);
            }
            _ => {}
        }
    }

    fn on_release(
        &mut self,
        prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        match (prev.last_perm_layer, prev.layer) {
            base_layer!(KeymapLayer::DvorakAnsi) => {
                keyboard_report_state.pop_key(KeyCode::SPACE);
            }
            base_layer!(KeymapLayer::DvorakSe | KeymapLayer::DvorakSeMac) => {
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
    fn on_press(&mut self, _keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        _keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
    }
    // Rotary encoder is here, no key
}

impl KeyboardButton for RightRow4Col2 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        keyboard_report_state.push_key(KeyCode::N2);
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        keyboard_report_state.pop_key(KeyCode::N2);
    }
}

impl KeyboardButton for RightRow4Col3 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        keyboard_report_state.push_key(KeyCode::N3);
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        keyboard_report_state.pop_key(KeyCode::N3);
    }
}

impl KeyboardButton for RightRow4Col4 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        keyboard_report_state.push_key(KeyCode::N4);
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        keyboard_report_state.pop_key(KeyCode::N4);
    }
}

impl KeyboardButton for RightRow4Col5 {
    fn on_press(&mut self, keyboard_report_state: &mut KeyboardReportState, _producer: &Producer) {
        keyboard_report_state.push_key(KeyCode::N5);
    }

    fn on_release(
        &mut self,
        _prev: LastPressState,
        keyboard_report_state: &mut KeyboardReportState,
        _producer: &Producer,
    ) {
        keyboard_report_state.pop_key(KeyCode::N5);
    }
}
