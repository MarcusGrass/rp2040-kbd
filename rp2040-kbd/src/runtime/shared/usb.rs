#[cfg(any(feature = "left", feature = "serial"))]
static mut USB_BUS: Option<usb_device::bus::UsbBusAllocator<liatris::hal::usb::UsbBus>> = None;

#[cfg(feature = "serial")]
static mut USB_DEVICE: Option<crate::keyboard::usb_serial::UsbSerialDevice> = None;

#[cfg(feature = "serial")]
static mut USB_SERIAL: Option<crate::keyboard::usb_serial::UsbSerial> = None;

#[cfg(feature = "hiddev")]
static mut USB_HID: Option<usbd_hid::hid_class::HIDClass<rp2040_hal::usb::UsbBus>> = None;

#[cfg(feature = "hiddev")]
static mut USB_HIDDEV: Option<usb_device::device::UsbDevice<rp2040_hal::usb::UsbBus>> = None;

#[cfg(feature = "serial")]
static mut USB_OUTPUT: bool = false;

#[cfg(feature = "serial")]
pub unsafe fn init_usb(allocator: usb_device::bus::UsbBusAllocator<liatris::hal::usb::UsbBus>) {
    USB_BUS = Some(allocator);
    // Ordering here is extremely important, serial before device.
    USB_SERIAL = Some(crate::keyboard::usb_serial::UsbSerial::new(
        USB_BUS.as_ref().unwrap(),
    ));
    USB_DEVICE = Some(crate::keyboard::usb_serial::UsbSerialDevice::new(
        USB_BUS.as_ref().unwrap(),
    ));
}

#[cfg(feature = "serial")]
pub fn acquire_usb<'a>() -> UsbGuard<'a> {
    let lock = crate::runtime::locks::UsbLock::claim();
    UsbGuard {
        serial: unsafe { USB_SERIAL.as_mut() },
        dev: unsafe { USB_DEVICE.as_mut() },
        output: unsafe { &mut USB_OUTPUT },
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
    USB_BUS = Some(allocator);

    let usb_hid = usbd_hid::hid_class::HIDClass::new_ep_in(
        USB_BUS.as_ref().unwrap(),
        usbd_hid::descriptor::KeyboardReport::desc(),
        1,
    );
    // Ordering here is extremely important, serial before device.
    USB_HID = Some(usb_hid);
    USB_HIDDEV = Some(
        usb_device::device::UsbDeviceBuilder::new(
            USB_BUS.as_ref().unwrap(),
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
