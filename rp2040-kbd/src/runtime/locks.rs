use rp2040_hal::sio::Spinlock14;

#[cfg(feature = "serial")]
pub type UsbLock = rp2040_hal::sio::Spinlock15;
pub type CrossCoreMsgLock = Spinlock14;
