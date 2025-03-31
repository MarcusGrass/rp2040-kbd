use crate::keyboard::oled::section::{OledRender, RightOledSide};
use crate::keyboard::oled::OledHandle;
use rp2040_hal::fugit::HertzU32;

pub struct RightOledDrawer {
    inner: OledRender<4, 3, RightOledSide>,
}

impl RightOledDrawer {
    pub fn new(handle: OledHandle) -> Self {
        Self {
            inner: OledRender::new(
                handle,
                &[
                    heapless::Vec::from_slice(&["RIGHT"]).unwrap(),
                    heapless::Vec::from_slice(&["PERF", "S ...", "P ..."]).unwrap(),
                    heapless::Vec::from_slice(&["DEBUG", "T ...", "Q ..."]).unwrap(),
                    heapless::Vec::from_slice(&["CLOCK", "..."]).unwrap(),
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
    pub fn update_touch(&mut self, transmitted: u16, avg_latency: f32) {
        self.inner
            .fmt_section(1, 2, format_args!("P {avg_latency:.1}"));
        self.inner
            .fmt_section(2, 1, format_args!("T {transmitted}"));
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
    pub fn render(&mut self) {
        self.inner.render();
    }

    #[inline]
    pub fn render_boot_msg(&mut self) {
        self.inner.render_boot_msg();
    }
}
