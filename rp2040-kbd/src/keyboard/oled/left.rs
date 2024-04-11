use crate::keyboard::oled::{DrawUnit, OledHandle};
use crate::static_draw_unit_string;
use core::fmt::Write;
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
    rx_header: DrawUnit,
    rx_content: DrawUnit,
    underscores_need_redraw: bool,
}

impl LeftOledDrawer {
    pub fn new(handle: OledHandle) -> Self {
        let header_content = static_draw_unit_string!("LEFT");
        let scan_loop_header_content = static_draw_unit_string!("SCAN");
        let scan_loop_unit = static_draw_unit_string!("...");
        let scan_loop_content = static_draw_unit_string!("...");
        let layer_header = static_draw_unit_string!("LAYER");
        let rx_header = static_draw_unit_string!("RX");
        let rx_content = static_draw_unit_string!("0");
        Self {
            handle,
            hidden: false,
            header: DrawUnit::new(header_content, true),
            scan_loop_header: DrawUnit::new(scan_loop_header_content, true),
            scan_loop_unit: DrawUnit::new(scan_loop_unit, true),
            scan_loop_content: DrawUnit::new(scan_loop_content, true),
            layer_header: DrawUnit::new(layer_header, true),
            layer_content: DrawUnit::new(static_draw_unit_string!(""), false),
            rx_header: DrawUnit::new(rx_header, true),
            rx_content: DrawUnit::new(rx_content, true),
            underscores_need_redraw: true,
        }
    }

    #[inline]
    pub fn hide(&mut self) {
        self.handle.clear();
        self.hidden = true;
    }

    #[inline]
    pub fn show(&mut self) {
        if self.hidden {
            self.header.needs_redraw = true;
            self.scan_loop_header.needs_redraw = true;
            self.scan_loop_content.needs_redraw = true;
            self.scan_loop_unit.needs_redraw = true;
            self.layer_header.needs_redraw = true;
            self.layer_content.needs_redraw = true;
            self.rx_header.needs_redraw = true;
            self.rx_content.needs_redraw = true;
            self.underscores_need_redraw = true;
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

    pub fn update_rx(&mut self, count: u16) {
        self.rx_content.content.clear();
        let _ = self.rx_content.content.write_fmt(format_args!("{count}"));
        self.rx_content.needs_redraw = true;
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
        if self.rx_header.needs_redraw {
            let _ = self.handle.clear_line(54);
            let _ = self.handle.write(54, self.rx_header.content.as_str());
            self.rx_header.needs_redraw = false;
        }
        if self.rx_content.needs_redraw {
            let _ = self.handle.clear_line(63);
            let _ = self.handle.write(63, self.rx_content.content.as_str());
            self.rx_content.needs_redraw = false;
        }
        if self.layer_header.needs_redraw {
            let _ = self.handle.clear_line(81);
            let _ = self.handle.write(81, self.layer_header.content.as_str());
            self.layer_header.needs_redraw = false;
        }
        if self.layer_content.needs_redraw {
            let _ = self.handle.clear_line(90);
            let _ = self.handle.write(90, self.layer_content.content.as_str());
            self.layer_content.needs_redraw = false;
        }
        if self.underscores_need_redraw {
            // Header
            let _ = self.handle.write_underscored_at(12);
            // Scan
            let _ = self.handle.write_underscored_at(48);
            // Rx
            let _ = self.handle.write_underscored_at(75);
            // Layer
            let _ = self.handle.write_underscored_at(102);
            self.underscores_need_redraw = false;
        }
    }

    pub fn render_boot_msg(&mut self) {
        self.handle.clear();
        let _ = self.handle.write(0, "LEFT");
        let _ = self.handle.write(9, "SIDE");
        let _ = self.handle.write(18, "ENTER");
        let _ = self.handle.write(27, "USB");
        let _ = self.handle.write(36, "BOOT");
    }
}
