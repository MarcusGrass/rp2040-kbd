use rp2040_hal::usb::UsbBus;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::UsbDevice;
use usb_device::UsbError;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
use usbd_hid::hid_class::HIDClass;

pub struct UsbHiddev<'a> {
    hid: HIDClass<'a, UsbBus>,
    dev: UsbDevice<'a, UsbBus>,
}

impl<'a> UsbHiddev<'a> {
    pub fn new(allocator: &'a UsbBusAllocator<UsbBus>) -> Self {
        let hid = usbd_hid::hid_class::HIDClass::new_ep_in(
            &allocator,
            usbd_hid::descriptor::KeyboardReport::desc(),
            1,
        );
        let dev = usb_device::device::UsbDeviceBuilder::new(
            &allocator,
            usb_device::device::UsbVidPid(0x16c0, 0x27da),
        )
        .manufacturer("Marcus Grass")
        .product("Lily58")
        .serial_number("1")
        .device_class(0)
        .build();
        Self { hid, dev }
    }

    pub fn submit_blocking(&mut self, keyboard_report: &KeyboardReport) -> bool {
        loop {
            match self.hid.push_input(keyboard_report) {
                Err(UsbError::WouldBlock) => while !self.poll() {},
                Ok(_) => {
                    break true;
                }
                Err(_) => {
                    break false;
                }
            }
        }
    }

    #[inline]
    pub fn poll(&mut self) -> bool {
        self.dev.poll(&mut [&mut self.hid])
    }
}
