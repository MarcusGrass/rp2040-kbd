use rp2040_hal::usb::UsbBus;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::UsbDevice;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
use usbd_hid::hid_class::HIDClass;

pub struct UsbHiddev<'a> {
    hid: HIDClass<'a, UsbBus>,
    dev: UsbDevice<'a, UsbBus>,
    ready: bool,
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
        .device_class(0)
        .build();
        Self {
            hid,
            dev,
            ready: true,
        }
    }

    pub fn try_submit_report(&mut self, keyboard_report: &KeyboardReport) -> bool {
        self.ready
            .then(|| {
                let res = self.hid.push_input(keyboard_report).is_ok();
                self.ready = false;
                res
            })
            .unwrap_or_default()
    }

    // Very easy to overproduce, only allow pushing after a previous poll, should come
    // from the OS-negotiated interrupt scheduling.
    // Could cache a value and immediately submit, but the producer
    // outpaces the os significantly so there's no need at the moment (42micros vs 1000 micros poll latency at time of writing)
    pub fn poll(&mut self) {
        self.dev.poll(&mut [&mut self.hid]);
        self.ready = true;
    }
}
