#[cfg(feature = "serial")]
pub type UsbLock = rp2040_hal::sio::Spinlock15;
#[cfg(feature = "right")]
pub type CrossCoreMsgLock = rp2040_hal::sio::Spinlock14;
