use core::borrow::BorrowMut;
use core::fmt::Write;
use rp2040_hal::usb::UsbBus;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::{StringDescriptors, UsbDevice, UsbDeviceBuilder, UsbVidPid};
use usb_device::UsbError;
use usbd_serial::SerialPort;

pub struct UsbSerial<'a> {
    pub(crate) inner: SerialPort<'a, UsbBus>,
}

impl<'a> UsbSerial<'a> {
    pub fn new(usb_bus: &'a UsbBusAllocator<UsbBus>) -> Self {
        // Set up the USB Communications Class Device driver
        let inner = SerialPort::new(usb_bus);
        Self { inner }
    }
}

impl Write for UsbSerial<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        serial_write_all(&mut self.inner, s.as_bytes());
        Ok(())
    }
}

fn serial_write_all<W: usb_device::bus::UsbBus, B1: BorrowMut<[u8]>, B2: BorrowMut<[u8]>>(
    serial: &mut SerialPort<W, B1, B2>,
    buf: &[u8],
) {
    for chunk in buf.chunks(16) {
        let mut remaining = chunk;
        loop {
            if remaining.is_empty() {
                break;
            }
            let written = serial.write(remaining);
            match written {
                Ok(wrote) => {
                    remaining = &remaining[wrote..];
                }
                Err(UsbError::WouldBlock) => {
                    continue;
                }
                Err(_e) => {
                    return;
                }
            }
        }
    }
}

pub struct UsbSerialDevice<'a> {
    pub(crate) inner: UsbDevice<'a, UsbBus>,
}

impl<'a> UsbSerialDevice<'a> {
    pub fn new(control_buffer: &'a mut [u8], usb_bus: &'a UsbBusAllocator<UsbBus>) -> Self {
        let inner = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27dd), control_buffer)
            .strings(&[StringDescriptors::default()
                .product("lily58")
                .manufacturer("splitkb")
                .serial_number("1")])
            .unwrap()
            .device_class(2) // from: https://www.usb.org/defined-class-codes
            .build()
            .unwrap();
        Self { inner }
    }
}
