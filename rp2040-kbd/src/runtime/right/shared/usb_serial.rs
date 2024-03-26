use core::cell::OnceCell;
use core::fmt::Write;
use core::marker::PhantomData;
use elite_pi::hal;
use rp2040_hal::sio::Spinlock15;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::UsbDevice;
use crate::keyboard::{INITIAL_STATE, MatrixState};
use crate::keyboard::usb_serial::{UsbSerial, UsbSerialDevice};

static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

static mut USB_DEVICE: Option<UsbSerialDevice> = None;

static mut USB_SERIAL: Option<UsbSerial> = None;

static mut USB_OUTPUT: bool = false;

pub unsafe fn init_usb(allocator: UsbBusAllocator<hal::usb::UsbBus>) {
    unsafe {
        USB_BUS = Some(allocator);
        // Ordering here is extremely important, serial before device.
        USB_SERIAL = Some(UsbSerial::new(USB_BUS.as_ref().unwrap()));
        USB_DEVICE = Some(UsbSerialDevice::new(USB_BUS.as_ref().unwrap()));
    }
}

pub fn acquire_usb<'a>() -> UsbGuard<'a> {
    let lock = Spinlock15::claim();
    UsbGuard {
        serial: unsafe {USB_SERIAL.as_mut()},
        dev: unsafe {USB_DEVICE.as_mut()},
        output: unsafe {&mut USB_OUTPUT},
        _lock: lock,
        _pd: Default::default(),
    }
}

pub struct UsbGuard<'a> {
    pub serial: Option<&'a mut UsbSerial<'static>>,
    pub dev: Option<&'a mut UsbSerialDevice<'static>>,
    pub output: &'a mut bool,
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


