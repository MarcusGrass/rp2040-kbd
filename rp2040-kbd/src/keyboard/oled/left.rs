use crate::keyboard::oled::{DrawUnit, OledHandle, OledWriter};
use heapless::String;

pub struct LeftOledDrawer {
    handle: OledHandle,
    hidden: bool,
    header: DrawUnit,
    scan_loop_header: DrawUnit,
    scan_loop_unit: DrawUnit,
    scan_loop_content: DrawUnit,
    layer_header: DrawUnit,
    layer_content: DrawUnit,
}

impl LeftOledDrawer {
    pub fn new(handle: OledHandle) -> Self {
        let mut header_content = String::new();
        let _ = header_content.push_str("LEFT");
        let mut scan_loop_header_content = String::new();
        let _ = scan_loop_header_content.push_str("SCAN");
        let mut layer_header = String::new();
        let _ = layer_header.push_str("LAYER");
        Self {
            handle,
            hidden: false,
            header: DrawUnit::new(header_content, true),
            scan_loop_header: DrawUnit::new(scan_loop_header_content, true),
            scan_loop_unit: DrawUnit::blank(),
            scan_loop_content: DrawUnit::blank(),
            layer_header: DrawUnit::new(layer_header, true),
            layer_content: DrawUnit::blank(),
        }
    }

    #[inline]
    pub fn hide(&mut self) {
        self.handle.clear();
        self.hidden = true;
    }

    #[inline]
    pub fn show(&mut self) {
        if self.hidden == true {
            self.header.needs_redraw = true;
            self.scan_loop_header.needs_redraw = true;
            self.scan_loop_content.needs_redraw = true;
            self.scan_loop_unit.needs_redraw = true;
            self.layer_header.needs_redraw = true;
            self.layer_content.needs_redraw = true;
        }
        self.hidden = false;
    }

    #[inline]
    pub fn update_scan_loop(&mut self, scan_loop_unit: String<5>, scan_loop_content: String<5>) {
        self.scan_loop_unit.content = scan_loop_unit;
        self.scan_loop_unit.needs_redraw = true;
        self.scan_loop_content.content = scan_loop_content;
        self.scan_loop_content.needs_redraw = true;
    }

    pub fn update_layer(&mut self, layer_content: String<5>) {
        self.layer_content.content = layer_content;
        self.layer_content.needs_redraw = true;
    }

    pub fn render(&mut self) {
        if self.hidden {
            return;
        }
        if self.header.needs_redraw {
            let _ = self.handle.clear_line(0);
            let _ = self.handle.write(0, self.header.content.as_str());
            self.header.needs_redraw = false;
        }
        if self.scan_loop_header.needs_redraw {
            let _ = self.handle.clear_line(18);
            let _ = self
                .handle
                .write(18, self.scan_loop_header.content.as_str());
            self.scan_loop_header.needs_redraw = false;
        }
        if self.scan_loop_content.needs_redraw {
            let _ = self.handle.clear_line(27);
            let _ = self
                .handle
                .write(27, self.scan_loop_content.content.as_str());
            self.scan_loop_content.needs_redraw = false;
        }
        if self.scan_loop_unit.needs_redraw {
            let _ = self.handle.clear_line(36);
            let _ = self.handle.write(36, self.scan_loop_unit.content.as_str());
            self.scan_loop_unit.needs_redraw = false;
        }
        if self.layer_header.needs_redraw {
            let _ = self.handle.clear_line(54);
            let _ = self.handle.write(54, self.layer_header.content.as_str());
            self.layer_header.needs_redraw = false;
        }
        if self.layer_content.needs_redraw {
            let _ = self.handle.clear_line(63);
            let _ = self.handle.write(63, self.layer_content.content.as_str());
            self.layer_content.needs_redraw = false;
        }
    }

    #[allow(dead_code)]
    pub fn render_boot_msg(&mut self) {
        self.handle.clear();
        let _ = self.handle.write(0, "BOOT");
    }
}

impl OledWriter for LeftOledDrawer {
    fn write_enter_boot_msg(&mut self) {
        self.handle.clear();
        let _ = self.handle.write(0, "LEFT");
        let _ = self.handle.write(18, "SIDE");
        let _ = self.handle.write(36, "ENTER");
        let _ = self.handle.write(54, "BOOT");
    }
}
