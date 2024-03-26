use elite_pi::hal;
use usb_device::bus::UsbBusAllocator;
use usb_device::device::UsbDevice;

static mut USB_BUS: Option<UsbBusAllocator<hal::usb::UsbBus>> = None;

static mut USB_DEVICE: Option<UsbDevice<hal::usb::UsbBus>> = None;

