pub const NUM_ROWS: u8 = 5;
pub const NUM_COLS: u8 = 6;

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct RowIndex(pub u8);

impl RowIndex {
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub const fn from_value(ind: u8) -> Self {
        assert!(
            ind < NUM_ROWS,
            "Tried to construct row index from a bad value"
        );
        Self(ind)
    }

    #[inline]
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct ColIndex(pub u8);

impl ColIndex {
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub const fn from_value(ind: u8) -> Self {
        assert!(
            ind < NUM_COLS,
            "Tried to construct col index from a bad value"
        );
        Self(ind)
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct MatrixIndex(u8);

impl MatrixIndex {
    #[inline]
    #[must_use]
    pub const fn from_row_col(row_index: RowIndex, col_index: ColIndex) -> Self {
        Self(row_index.0 * NUM_COLS + col_index.0)
    }

    #[must_use]
    #[inline(always)]
    pub const fn byte(&self) -> u8 {
        self.0
    }

    #[must_use]
    #[inline(always)]
    pub const fn index(&self) -> usize {
        self.0 as usize
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone)]
pub struct MatrixUpdate(u8);

#[derive(Debug, Copy, Clone)]
pub enum MatrixChange {
    KeyUpdate(MatrixIndex, bool),
    EncoderUpdate(bool),
}

impl MatrixUpdate {
    const KEY_STATE_BIT: u8 = 0b0010_0000;

    const KEY_INDEX_MASK: u8 = 0b0001_1111;
    const ENCODER_STATE_BIT: u8 = 0b1000_0000;
    const ENCODER_ON: Self = Self(0b1100_0000);
    const ENCODER_OFF: Self = Self(0b0100_0000);
    #[must_use]
    pub fn from_byte(byte: u8) -> Option<Self> {
        let ind = byte & Self::KEY_INDEX_MASK;
        // There are 2 illegal bit-patterns
        (ind <= 30).then_some(Self(byte))
    }

    #[inline]
    #[must_use]
    pub const fn from_key_update(index: MatrixIndex, state: bool) -> Self {
        let mut val = index.0;
        if state {
            val |= Self::KEY_STATE_BIT;
        }
        Self(val)
    }

    #[inline]
    #[must_use]
    pub const fn from_rotary_change(clockwise: bool) -> Self {
        if clockwise {
            Self::ENCODER_ON
        } else {
            Self::ENCODER_OFF
        }
    }

    #[inline]
    #[must_use]
    pub const fn interpret_byte(&self) -> MatrixChange {
        let encoder = self.0 & Self::ENCODER_ON.0;
        if encoder == 0 {
            let idx = self.0 & Self::KEY_INDEX_MASK;
            let state = self.0 & Self::KEY_STATE_BIT;
            MatrixChange::KeyUpdate(MatrixIndex(idx), state != 0)
        } else {
            MatrixChange::EncoderUpdate(encoder & Self::ENCODER_STATE_BIT != 0)
        }
    }

    #[inline]
    #[must_use]
    pub const fn byte(self) -> u8 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_from_key() {
        const R1: RowIndex = RowIndex::from_value(1);
        const C1: ColIndex = ColIndex::from_value(4);
        const M1: MatrixIndex = MatrixIndex::from_row_col(R1, C1);
        const MU1: MatrixUpdate = MatrixUpdate::from_key_update(M1, true);
        assert_eq!(0b00101010, MU1.0, "{:b}", MU1.0);
        const EXPECT_IND: u8 = R1.0 * NUM_COLS + C1.0;
        assert!(matches!(
            MU1.interpret_byte(),
            MatrixChange::KeyUpdate(MatrixIndex(EXPECT_IND), true)
        ));
        const MU2: MatrixUpdate = MatrixUpdate::from_key_update(M1, false);
        assert!(matches!(
            MU2.interpret_byte(),
            MatrixChange::KeyUpdate(MatrixIndex(EXPECT_IND), false)
        ));
        assert_eq!(0b00001010, MU2.0, "{:b}", MU2.0);
    }
}
