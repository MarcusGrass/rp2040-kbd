use crate::keyboard::oled::{DrawUnit, OledHandle};
use crate::static_draw_unit_string;
use core::fmt::Write;
use heapless::String;

pub struct RightOledDrawer {
    handle: OledHandle,
    hidden: bool,
    header: DrawUnit,
    scan_loop_header: DrawUnit,
    scan_loop_unit: DrawUnit,
    scan_loop_content: DrawUnit,
    tx_header: DrawUnit,
    tx_content: DrawUnit,
    underscores_need_redraw: bool,
}

impl RightOledDrawer {
    pub fn new(handle: OledHandle) -> Self {
        let header_content = static_draw_unit_string!("RIGHT");
        let scan_loop_header_content = static_draw_unit_string!("SCAN");
        let scan_loop_unit = static_draw_unit_string!("...");
        let scan_loop_content = static_draw_unit_string!("...");
        let tx_header = static_draw_unit_string!("TX");
        let tx_content = static_draw_unit_string!("0");
        Self {
            handle,
            hidden: false,
            header: DrawUnit::new(header_content, true),
            scan_loop_header: DrawUnit::new(scan_loop_header_content, true),
            scan_loop_unit: DrawUnit::new(scan_loop_unit, true),
            scan_loop_content: DrawUnit::new(scan_loop_content, true),
            tx_header: DrawUnit::new(tx_header, true),
            tx_content: DrawUnit::new(tx_content, true),
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
            self.tx_header.needs_redraw = true;
            self.tx_content.needs_redraw = true;
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

    pub fn update_tx(&mut self, count: u16) {
        self.tx_content.content.clear();
        let _ = self.tx_content.content.write_fmt(format_args!("{count}"));
        self.tx_content.needs_redraw = true;
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
        if self.tx_header.needs_redraw {
            let _ = self.handle.clear_line(54);
            let _ = self.handle.write(54, self.tx_header.content.as_str());
            self.tx_header.needs_redraw = false;
        }
        if self.tx_content.needs_redraw {
            let _ = self.handle.clear_line(63);
            let _ = self.handle.write(63, self.tx_content.content.as_str());
            self.tx_content.needs_redraw = false;
        }
        if self.underscores_need_redraw {
            // Header
            let _ = self.handle.write_underscored_at(12);
            // Scan
            let _ = self.handle.write_underscored_at(48);
            // Tx
            let _ = self.handle.write_underscored_at(75);
            self.underscores_need_redraw = false;
        }
    }

    pub fn render_boot_msg(&mut self) {
        self.handle.clear();
        let _ = self.handle.write(0, "RIGHT");
        let _ = self.handle.write(9, "SIDE");
        let _ = self.handle.write(18, "ENTER");
        let _ = self.handle.write(27, "USB");
        let _ = self.handle.write(36, "BOOT");
    }
}
