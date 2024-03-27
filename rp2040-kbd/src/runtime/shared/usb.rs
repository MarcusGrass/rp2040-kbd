use crate::keyboard::usb_serial::{UsbSerial, UsbSerialDevice};
use crate::keyboard::{MatrixState, INITIAL_STATE};
use core::cell::OnceCell;
use core::fmt::Write;
use core::marker::PhantomData;
use liatris::hal;
use rp2040_hal::sio::{Spinlock14, Spinlock15};
use rp2040_hal::usb::UsbBus;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
use usbd_hid::descriptor::{KeyboardReport, MouseReport, SerializedDescriptor};
use usbd_hid::hid_class::HIDClass;

static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

static mut USB_DEVICE: Option<UsbSerialDevice> = None;

#[cfg(feature = "serial")]
static mut USB_SERIAL: Option<UsbSerial> = None;

#[cfg(feature = "hiddev")]
static mut USB_HID: Option<HIDClass<UsbBus>> = None;

static mut USB_HIDDEV: Option<UsbDevice<UsbBus>> = None;

static mut USB_OUTPUT: bool = false;

#[cfg(feature = "serial")]
pub unsafe fn init_usb(allocator: UsbBusAllocator<hal::usb::UsbBus>) {
    USB_BUS = Some(allocator);
    // Ordering here is extremely important, serial before device.
    USB_SERIAL = Some(UsbSerial::new(USB_BUS.as_ref().unwrap()));
    USB_DEVICE = Some(UsbSerialDevice::new(USB_BUS.as_ref().unwrap()));
}


#[cfg(feature = "serial")]
pub fn acquire_usb<'a>() -> UsbGuard<'a> {
    let lock = Spinlock15::claim();
    UsbGuard {
        serial: unsafe { USB_SERIAL.as_mut() },
        dev: unsafe { USB_DEVICE.as_mut() },
        output: unsafe { &mut USB_OUTPUT },
        _lock: lock,
        _pd: PhantomData::default()
    }
}

#[cfg(feature = "hiddev")]
pub fn acquire_usb<'a>() -> UsbGuard<'a> {
    UsbGuard {
        serial: None,
        dev: None,
        output: unsafe { &mut USB_OUTPUT },
        _pd: PhantomData::default()

    }
}

pub struct UsbGuard<'a> {
    pub serial: Option<&'a mut UsbSerial<'static>>,
    pub dev: Option<&'a mut UsbSerialDevice<'static>>,
    pub output: &'a mut bool,
    #[cfg(feature = "serial")]
    _lock: Spinlock15,
    _pd: PhantomData<&'a ()>,
}


impl<'a> Write for UsbGuard<'a> {
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
pub unsafe fn init_usb(allocator: UsbBusAllocator<hal::usb::UsbBus>) {
    USB_BUS = Some(allocator);

    let usb_hid = HIDClass::new(USB_BUS.as_ref().unwrap(), KeyboardReport::desc(), 5);
    // Ordering here is extremely important, serial before device.
    USB_HID = Some(usb_hid);
    USB_HIDDEV = Some(UsbDeviceBuilder::new(USB_BUS.as_ref().unwrap(), UsbVidPid(0x16c0, 0x27da))
        .manufacturer("Marcus Grass")
        .product("Lily58")
        .serial_number("1")
        .device_class(0)
        .build()
    );
}
pub fn push_hid_report(keyboard_report: KeyboardReport) {
    critical_section::with(|_| unsafe {
        Spinlock14::claim();
        USB_HID.as_mut().map(|hid| hid.push_input(&keyboard_report))
    });
}

#[inline]
pub fn usb_hid_interrupt_poll() {
    Spinlock14::claim();
    unsafe {
        if let (Some(dev), Some(hid)) = (USB_HIDDEV.as_mut(), USB_HID.as_mut()) {
            dev.poll(&mut [hid]);
        }
    }
}
