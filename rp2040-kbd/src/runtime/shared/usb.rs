pub struct SyncBus(
    core::cell::OnceCell<usb_device::bus::UsbBusAllocator<liatris::hal::usb::UsbBus>>,
);

unsafe impl Sync for SyncBus {}

pub struct SyncUnsafe<T>(core::cell::UnsafeCell<T>);

unsafe impl<T> Sync for SyncUnsafe<T> where T: Sync {}

pub struct SyncUnsafeOnce<T>(core::cell::OnceCell<SyncUnsafe<T>>);

unsafe impl<T> Sync for SyncUnsafeOnce<T> where T: Sync {}

impl<T> SyncUnsafeOnce<T> {
    const fn new() -> Self {
        Self(core::cell::OnceCell::new())
    }

    fn set(&self, val: T) {
        let _ = self.0.set(SyncUnsafe(core::cell::UnsafeCell::new(val)));
    }

    /// # Safety
    /// Only a single reference to this is held
    #[inline]
    pub unsafe fn as_mut<'a>(&'static self) -> Option<&'a mut T> {
        self.0.get().and_then(|r| r.0.get().as_mut())
    }
}
static USB_BUS: SyncBus = SyncBus(core::cell::OnceCell::new());

static USB_DEVICE: SyncUnsafeOnce<crate::keyboard::usb_serial::UsbSerialDevice> =
    SyncUnsafeOnce::new();

static USB_SERIAL: SyncUnsafeOnce<crate::keyboard::usb_serial::UsbSerial> = SyncUnsafeOnce::new();

static USB_OUTPUT: SyncUnsafeOnce<bool> = SyncUnsafeOnce::new();

pub unsafe fn init_usb(allocator: usb_device::bus::UsbBusAllocator<liatris::hal::usb::UsbBus>) {
    let _ = USB_BUS.0.set(allocator);
    USB_OUTPUT.set(false);
    // Ordering here is extremely important, serial before device.
    USB_SERIAL.set(crate::keyboard::usb_serial::UsbSerial::new(
        USB_BUS.0.get().unwrap(),
    ));
    USB_DEVICE.set(crate::keyboard::usb_serial::UsbSerialDevice::new(
        USB_BUS.0.get().unwrap(),
    ));
}

pub fn acquire_usb<'a>() -> UsbGuard<'a> {
    let lock = crate::runtime::locks::UsbLock::claim();
    UsbGuard {
        serial: unsafe { USB_SERIAL.as_mut() },
        dev: unsafe { USB_DEVICE.as_mut() },
        output: unsafe { USB_OUTPUT.as_mut().unwrap() },
        _lock: lock,
        _pd: core::marker::PhantomData::default(),
    }
}

pub struct UsbGuard<'a> {
    pub serial: Option<&'a mut crate::keyboard::usb_serial::UsbSerial<'static>>,
    pub dev: Option<&'a mut crate::keyboard::usb_serial::UsbSerialDevice<'static>>,
    pub output: &'a mut bool,
    _lock: crate::runtime::locks::UsbLock,
    _pd: core::marker::PhantomData<&'a ()>,
}

impl<'a> core::fmt::Write for UsbGuard<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if let Some(serial) = self.serial.as_mut() {
            if *self.output {
                serial.write_str(s)
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }
}
