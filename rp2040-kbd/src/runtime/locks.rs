use rp2040_hal::sio::{Spinlock14, Spinlock15};

pub type UsbLock = Spinlock15;
pub type CrossCoreMsgLock = Spinlock14;
