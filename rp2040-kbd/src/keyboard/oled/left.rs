use crate::keyboard::oled::section::{LeftOledSide, OledRender};
use crate::keyboard::oled::{OledHandle, OledLineString};
use rp2040_hal::fugit::HertzU32;

pub struct LeftOledDrawer {
    inner: OledRender<5, 4, LeftOledSide>,
}

impl LeftOledDrawer {
    pub fn new(handle: OledHandle) -> Self {
        Self {
            inner: OledRender::new(
                handle,
                &[
                    heapless::Vec::from_slice(&["LEFT"]).unwrap(),
                    heapless::Vec::from_slice(&["PERF", "S ...", "L ...", "R ..."]).unwrap(),
                    heapless::Vec::from_slice(&["DEBUG", "R ...", "Q ..."]).unwrap(),
                    heapless::Vec::from_slice(&["CLOCK", "..."]).unwrap(),
                    heapless::Vec::from_slice(&["LAYER", "..."]).unwrap(),
                ],
            ),
        }
    }

    #[inline]
    pub fn hide(&mut self) {
        self.inner.hide();
    }

    #[inline]
    pub fn show(&mut self) {
        self.inner.show();
    }

    #[inline]
    pub fn update_scan_loop(&mut self, avg_scan_latency: f32) {
        self.inner
            .fmt_section(1, 1, format_args!("S {avg_scan_latency:.1}"));
    }

    #[inline]
    pub fn update_left_counter(&mut self, avg_latency: f32) {
        self.inner
            .fmt_section(1, 2, format_args!("L {avg_latency:.1}"));
    }

    #[inline]
    pub fn update_right_counter(&mut self, avg_latency: f32) {
        self.inner
            .fmt_section(1, 3, format_args!("R {avg_latency:.1}"));
    }

    #[inline]
    pub fn update_rx(&mut self, count: u16) {
        self.inner.fmt_section(2, 1, format_args!("R {count}"));
    }

    #[inline]
    pub fn update_queue(&mut self, count: usize) {
        self.inner.fmt_section(2, 2, format_args!("Q {count}"));
    }

    #[inline]
    pub fn set_clock(&mut self, freq: HertzU32) {
        self.inner
            .fmt_section(3, 1, format_args!("{}Mhz", freq.to_MHz()));
    }

    #[inline]
    pub fn update_layer(&mut self, default_layer: OledLineString) {
        self.inner.set_section(4, 1, default_layer);
    }

    #[inline]
    pub fn render(&mut self) {
        self.inner.render();
    }

    #[inline]
    pub fn render_boot_msg(&mut self) {
        self.inner.render_boot_msg();
    }
}

pub fn layer_to_string(keymap_layer: crate::keymap::KeymapLayer) -> OledLineString {
    let mut s = heapless::String::new();
    match keymap_layer {
        crate::keymap::KeymapLayer::DvorakSe => {
            let _ = s.push_str("DV-SE");
        }
        crate::keymap::KeymapLayer::DvorakAnsi => {
            let _ = s.push_str("DV-AN");
        }
        crate::keymap::KeymapLayer::QwertyGaming => {
            let _ = s.push_str("QW-GM");
        }
        crate::keymap::KeymapLayer::Lower => {
            let _ = s.push_str("LO");
        }
        crate::keymap::KeymapLayer::LowerAnsi => {
            let _ = s.push_str("LO-AN");
        }
        crate::keymap::KeymapLayer::Raise => {
            let _ = s.push_str("RA");
        }
        crate::keymap::KeymapLayer::Num => {
            let _ = s.push_str("NUM");
        }
        crate::keymap::KeymapLayer::Settings => {
            let _ = s.push_str("SET");
        }
    }
    s
}
