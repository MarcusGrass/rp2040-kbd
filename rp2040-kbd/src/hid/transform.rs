use usbd_hid::descriptor::KeyboardReport;

pub fn transform_state() -> KeyboardReport {
    KeyboardReport {
        modifier: 0,
        reserved: 0,
        leds: 0,
        keycodes: [0u8; 6],
    }
}
