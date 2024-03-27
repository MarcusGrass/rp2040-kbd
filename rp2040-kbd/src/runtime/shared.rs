pub mod usb;

use crate::keyboard::{MatrixState, INITIAL_STATE};
use core::marker::PhantomData;
use rp2040_hal::sio::Spinlock0;

static mut MATRIX_SCAN: MatrixScan = MatrixScan { num_scans: 0 };

pub struct MatrixScanGuard<'a> {
    pub scan: &'static mut MatrixScan,
    _lock: Spinlock0,
    _pd: PhantomData<&'a ()>,
}

pub fn try_acquire_matrix_scan<'a>() -> Option<MatrixScanGuard<'a>> {
    let lock = Spinlock0::try_claim()?;
    Some(MatrixScanGuard {
        scan: unsafe { &mut MATRIX_SCAN },
        _lock: lock,
        _pd: Default::default(),
    })
}

pub fn acquire_matrix_scan<'a>() -> MatrixScanGuard<'a> {
    let lock = Spinlock0::claim();
    MatrixScanGuard {
        scan: unsafe { &mut MATRIX_SCAN },
        _lock: lock,
        _pd: Default::default(),
    }
}
#[derive(Debug, Copy, Clone)]
pub struct MatrixScan {
    pub num_scans: usize,
}
