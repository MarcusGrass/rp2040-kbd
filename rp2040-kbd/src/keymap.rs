use crate::hid::keycodes::{KeyCode, Modifier};
use crate::keyboard::{matrix_ind, MatrixState, NUM_COLS, NUM_ROWS};
use usbd_hid::descriptor::KeyboardReport;

#[derive(Debug, Copy, Clone)]
pub enum Layers {
    DvorakAnsi,
}

#[derive(Debug, Copy, Clone)]
pub struct LayerResult {
    pub next_layer: Option<Layers>,
    pub report: KeyboardReport,
}

impl Layers {
    pub fn report(self, left: &MatrixState, right: &MatrixState) -> LayerResult {
        match self {
            Layers::DvorakAnsi => dvorak_se_to_report(left, right),
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
    at_ind_keycode!(left, 0, 2, keycodes, code_ind, KeyCode::KC_COMM);
    at_ind_keycode!(left, 0, 3, keycodes, code_ind, KeyCode::KC_DOT);
    at_ind_keycode!(left, 0, 4, keycodes, code_ind, KeyCode::KC_P);
    at_ind_keycode!(left, 0, 5, keycodes, code_ind, KeyCode::KC_Y);

    at_ind_keycode!(left, 1, 0, keycodes, code_ind, KeyCode::KC_ESC);
    at_ind_keycode!(left, 1, 1, keycodes, code_ind, KeyCode::KC_A);
    at_ind_keycode!(left, 1, 2, keycodes, code_ind, KeyCode::KC_O);
    at_ind_keycode!(left, 1, 3, keycodes, code_ind, KeyCode::KC_E);
    at_ind_keycode!(left, 1, 4, keycodes, code_ind, KeyCode::KC_U);
    at_ind_keycode!(left, 1, 5, keycodes, code_ind, KeyCode::KC_I);

    at_ind_mod!(left, 2, 0, mods, Modifier::KC_LSHIFT);
    at_ind_keycode!(left, 2, 1, keycodes, code_ind, KeyCode::KC_SEMC);
    at_ind_keycode!(left, 2, 2, keycodes, code_ind, KeyCode::KC_Q);
    at_ind_keycode!(left, 2, 3, keycodes, code_ind, KeyCode::KC_J);
    at_ind_keycode!(left, 2, 4, keycodes, code_ind, KeyCode::KC_K);
    at_ind_keycode!(left, 2, 5, keycodes, code_ind, KeyCode::KC_X);

    at_ind_mod!(left, 3, 0, mods, Modifier::KC_LCTRL);
    at_ind_keycode!(left, 3, 1, keycodes, code_ind, KeyCode::KC_SEMC);
    at_ind_keycode!(left, 3, 2, keycodes, code_ind, KeyCode::KC_Q);
    at_ind_keycode!(left, 3, 3, keycodes, code_ind, KeyCode::KC_J);
    at_ind_keycode!(left, 3, 4, keycodes, code_ind, KeyCode::KC_K);
    at_ind_keycode!(left, 3, 5, keycodes, code_ind, KeyCode::KC_SPC);

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

    at_ind_mod!(right, 2, 0, mods, Modifier::KC_LSHIFT);
    at_ind_keycode!(right, 2, 1, keycodes, code_ind, KeyCode::KC_Z);
    at_ind_keycode!(right, 2, 2, keycodes, code_ind, KeyCode::KC_V);
    at_ind_keycode!(right, 2, 3, keycodes, code_ind, KeyCode::KC_W);
    at_ind_keycode!(right, 2, 4, keycodes, code_ind, KeyCode::KC_M);
    at_ind_keycode!(right, 2, 5, keycodes, code_ind, KeyCode::KC_B);

    // Unused, encoder is at 0
    at_ind_mod!(right, 3, 0, mods, Modifier::KC_LCTRL);
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
