use crate::keyboard::oled::{DrawUnit, OledHandle};
use crate::static_draw_unit_string;
use core::fmt::Write;
use rp2040_hal::fugit::HertzU32;

pub struct RightOledDrawer {
    handle: OledHandle,
    hidden: bool,
    header: DrawUnit,
    scan_loop_header: DrawUnit,
    scan_loop_content: DrawUnit,
    press_loop_content: DrawUnit,
    dbg_header: DrawUnit,
    dbg_tx: DrawUnit,
    dbg_queue: DrawUnit,
    clk_header: DrawUnit,
    clk_freq: DrawUnit,
    underscores_need_redraw: bool,
}

impl RightOledDrawer {
    pub fn new(handle: OledHandle) -> Self {
        let header_content = static_draw_unit_string!("RIGHT");
        let scan_loop_header_content = static_draw_unit_string!("PERF");
        let scan_loop_content = static_draw_unit_string!("S ...");
        let press_loop_content = static_draw_unit_string!("P ...");
        let dbg_header = static_draw_unit_string!("DEBUG");
        let dbg_tx = static_draw_unit_string!("T ...");
        let dbg_queue = static_draw_unit_string!("Q ...");
        let clk_header = static_draw_unit_string!("CLOCK");
        let clk_freq = static_draw_unit_string!("...");
        Self {
            handle,
            hidden: false,
            header: DrawUnit::new(header_content, true),
            scan_loop_header: DrawUnit::new(scan_loop_header_content, true),
            scan_loop_content: DrawUnit::new(scan_loop_content, true),
            press_loop_content: DrawUnit::new(press_loop_content, true),
            dbg_header: DrawUnit::new(dbg_header, true),
            dbg_tx: DrawUnit::new(dbg_tx, true),
            dbg_queue: DrawUnit::new(dbg_queue, true),
            clk_header: DrawUnit::new(clk_header, true),
            clk_freq: DrawUnit::new(clk_freq, true),
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
            self.press_loop_content.needs_redraw = true;
            self.dbg_header.needs_redraw = true;
            self.dbg_tx.needs_redraw = true;
            self.dbg_queue.needs_redraw = true;
            self.clk_header.needs_redraw = true;
            self.clk_freq.needs_redraw = true;
            self.underscores_need_redraw = true;
        }
        self.hidden = false;
    }

    pub fn update_scan_loop(&mut self, avg_scan_latency: f32) {
        self.scan_loop_content.content.clear();
        let _ = self
            .scan_loop_content
            .content
            .write_fmt(format_args!("S {avg_scan_latency:.1}"));
        self.scan_loop_content.needs_redraw = true;
    }

    pub fn update_touch(&mut self, transmitted: u16, avg_latency: f32) {
        self.press_loop_content.content.clear();
        let _ = self
            .press_loop_content
            .content
            .write_fmt(format_args!("P {avg_latency:.1}"));
        self.press_loop_content.needs_redraw = true;
        self.dbg_tx.content.clear();
        let _ = self
            .dbg_tx
            .content
            .write_fmt(format_args!("T {transmitted}"));
        self.dbg_tx.needs_redraw = true;
    }
    pub fn update_queue(&mut self, count: usize) {
        self.dbg_queue.content.clear();
        let _ = self.dbg_queue.content.write_fmt(format_args!("Q {count}"));
        self.dbg_queue.needs_redraw = true;
    }

    pub fn set_clock(&mut self, freq: HertzU32) {
        self.clk_freq.content.clear();
        let _ = self
            .clk_freq
            .content
            .write_fmt(format_args!("{}Mhz", freq.to_MHz()));
        self.clk_freq.needs_redraw = true;
    }

    pub fn render(&mut self) {
        if self.hidden {
            return;
        }
        if self.header.needs_redraw {
            let _ = self.handle.clear_line(0);
            let _ = self.handle.write_header(0, self.header.content.as_str());
            self.header.needs_redraw = false;
        }
        if self.scan_loop_header.needs_redraw {
            let _ = self.handle.clear_line(12);
            let _ = self
                .handle
                .write_header(12, self.scan_loop_header.content.as_str());
            self.scan_loop_header.needs_redraw = false;
        }
        if self.scan_loop_content.needs_redraw {
            let _ = self.handle.clear_line(20);
            let _ = self
                .handle
                .write_header(20, self.scan_loop_content.content.as_str());
            self.scan_loop_content.needs_redraw = false;
        }
        if self.press_loop_content.needs_redraw {
            let _ = self.handle.clear_line(28);
            let _ = self
                .handle
                .write_header(28, self.press_loop_content.content.as_str());
            self.press_loop_content.needs_redraw = false;
        }
        if self.dbg_header.needs_redraw {
            let _ = self.handle.clear_line(40);
            let _ = self
                .handle
                .write_header(40, self.dbg_header.content.as_str());
            self.dbg_header.needs_redraw = false;
        }
        if self.dbg_tx.needs_redraw {
            let _ = self.handle.clear_line(48);
            let _ = self.handle.write_header(48, self.dbg_tx.content.as_str());
            self.dbg_tx.needs_redraw = false;
        }
        if self.dbg_queue.needs_redraw {
            let _ = self.handle.clear_line(56);
            let _ = self
                .handle
                .write_header(56, self.dbg_queue.content.as_str());
            self.dbg_queue.needs_redraw = false;
        }
        if self.clk_header.needs_redraw {
            let _ = self.handle.clear_line(68);
            let _ = self
                .handle
                .write_header(68, self.clk_header.content.as_str());
            self.clk_header.needs_redraw = false;
        }
        if self.clk_freq.needs_redraw {
            let _ = self.handle.clear_line(76);
            let _ = self.handle.write_header(76, self.clk_freq.content.as_str());
            self.clk_freq.needs_redraw = false;
        }
        if self.underscores_need_redraw {
            // Header
            let _ = self.handle.write_underscored_at(8);
            // Perf
            let _ = self.handle.write_underscored_at(36);
            // Dbg
            let _ = self.handle.write_underscored_at(64);
            // Clock
            let _ = self.handle.write_underscored_at(84);
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
