#[cfg(any(feature = "left", feature = "serial"))]
pub struct SyncBus(
    core::cell::OnceCell<usb_device::bus::UsbBusAllocator<liatris::hal::usb::UsbBus>>,
);

#[cfg(any(feature = "left", feature = "serial"))]
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
#[cfg(any(feature = "left", feature = "serial"))]
static USB_BUS: SyncBus = SyncBus(core::cell::OnceCell::new());

#[cfg(feature = "serial")]
static USB_DEVICE: SyncUnsafeOnce<crate::keyboard::usb_serial::UsbSerialDevice> =
    SyncUnsafeOnce::new();

#[cfg(feature = "serial")]
static USB_SERIAL: SyncUnsafeOnce<crate::keyboard::usb_serial::UsbSerial> = SyncUnsafeOnce::new();

#[cfg(feature = "hiddev")]
static USB_HID: SyncUnsafeOnce<usbd_hid::hid_class::HIDClass<rp2040_hal::usb::UsbBus>> =
    SyncUnsafeOnce::new();

#[cfg(feature = "hiddev")]
static USB_HIDDEV: SyncUnsafeOnce<usb_device::device::UsbDevice<rp2040_hal::usb::UsbBus>> =
    SyncUnsafeOnce::new();

#[cfg(feature = "serial")]
static USB_OUTPUT: SyncUnsafeOnce<bool> = SyncUnsafeOnce::new();

#[cfg(feature = "serial")]
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

#[cfg(feature = "serial")]
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

#[cfg(feature = "serial")]
pub struct UsbGuard<'a> {
    pub serial: Option<&'a mut crate::keyboard::usb_serial::UsbSerial<'static>>,
    pub dev: Option<&'a mut crate::keyboard::usb_serial::UsbSerialDevice<'static>>,
    pub output: &'a mut bool,
    _lock: crate::runtime::locks::UsbLock,
    _pd: core::marker::PhantomData<&'a ()>,
}

#[cfg(feature = "serial")]
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

#[cfg(feature = "hiddev")]
pub unsafe fn init_usb(allocator: usb_device::bus::UsbBusAllocator<liatris::hal::usb::UsbBus>) {
    use usbd_hid::descriptor::SerializedDescriptor;
    let _ = USB_BUS.0.set(allocator);

    let usb_hid = usbd_hid::hid_class::HIDClass::new_ep_in(
        USB_BUS.0.get().unwrap(),
        usbd_hid::descriptor::KeyboardReport::desc(),
        1,
    );
    // Ordering here is extremely important, serial before device.
    USB_HID.set(usb_hid);
    USB_HIDDEV.set(
        usb_device::device::UsbDeviceBuilder::new(
            USB_BUS.0.get().unwrap(),
            usb_device::device::UsbVidPid(0x16c0, 0x27da),
        )
        .manufacturer("Marcus Grass")
        .product("Lily58")
        .serial_number("1")
        .device_class(0)
        .build(),
    );
}

#[cfg(feature = "hiddev")]
pub fn push_hid_report(keyboard_report: &usbd_hid::descriptor::KeyboardReport) -> bool {
    critical_section::with(|_| unsafe {
        !matches!(
            USB_HID.as_mut().map(|hid| hid.push_input(keyboard_report)),
            Some(Err(usb_device::UsbError::WouldBlock))
        )
    })
}

#[inline]
#[cfg(feature = "hiddev")]
pub fn usb_hid_interrupt_poll() {
    unsafe {
        if let (Some(dev), Some(hid)) = (USB_HIDDEV.as_mut(), USB_HID.as_mut()) {
            dev.poll(&mut [hid]);
        }
    }
}
