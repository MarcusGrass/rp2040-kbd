#[cfg(feature = "serial")]
pub type UsbLock = rp2040_hal::sio::Spinlock15;
