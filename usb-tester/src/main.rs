use std::time::Duration;
use libusb::DeviceHandle;

fn main() {
    let ctx = libusb::Context::new().unwrap();
    for dev in ctx.devices().unwrap().iter() {
        let dev_desc = dev.device_descriptor().unwrap();
        // My dummy ids
        if dev_desc.vendor_id() != 0x16c0 || dev_desc.product_id() != 0x27dd {
            continue;
        }
        let cfg = dev.active_config_descriptor().unwrap();
        for it in cfg.interfaces().into_iter() {
            for desc in it.descriptors() {
                for ep in desc.endpoint_descriptors() {
                    println!("{:?} -> {ep:?}", desc);
                }
            }
        }
        let handle = dev.open().unwrap();
        use_dev_handle(handle);
    }
}

fn use_dev_handle(mut handle: DeviceHandle) {
    handle.claim_interface(0).unwrap();
    let write = handle.write_bulk(1, b"Hello!", Duration::from_secs(3)).unwrap();
    println!("Wrote {write} bytes");
    let mut buf = vec![0; 2048];
    handle.claim_interface(0).unwrap();
    let read = handle.read_bulk(129, buf.as_mut_slice(), Duration::from_secs(3)).unwrap();
    println!("Read {} bytes", read);
    println!("{:?}", core::str::from_utf8(&buf[..read]));
}