use crate::keyboard::oled::{DrawUnit, OledHandle, OledLineString};
use crate::static_draw_unit_string;
use core::fmt::Write;
use rp2040_hal::fugit::HertzU32;

pub struct LeftOledDrawer {
    handle: OledHandle,
    hidden: bool,
    header: DrawUnit,
    scan_loop_header: DrawUnit,
    scan_loop_content: DrawUnit,
    press_left_loop_content: DrawUnit,
    press_right_loop_content: DrawUnit,
    dbg_header: DrawUnit,
    dbg_rx: DrawUnit,
    dbg_queue: DrawUnit,
    clk_header: DrawUnit,
    clk_freq: DrawUnit,
    layer_header: DrawUnit,
    perm_layer: DrawUnit,
    underscores_need_redraw: bool,
}

impl LeftOledDrawer {
    pub fn new(handle: OledHandle) -> Self {
        let header_content = static_draw_unit_string!("LEFT");
        let scan_loop_header_content = static_draw_unit_string!("PERF");
        let scan_loop_content = static_draw_unit_string!("S ...");
        let press_left_loop_content = static_draw_unit_string!("L ...");
        let press_right_loop_content = static_draw_unit_string!("R ...");
        let dbg_header = static_draw_unit_string!("DEBUG");
        let dbg_rx = static_draw_unit_string!("R ...");
        let dbg_queue = static_draw_unit_string!("Q ...");
        let clk_header = static_draw_unit_string!("CLOCK");
        let clk_freq = static_draw_unit_string!("...");
        let layer_header = static_draw_unit_string!("LAYER");
        Self {
            handle,
            hidden: false,
            header: DrawUnit::new(header_content, true),
            scan_loop_header: DrawUnit::new(scan_loop_header_content, true),
            scan_loop_content: DrawUnit::new(scan_loop_content, true),
            press_left_loop_content: DrawUnit::new(press_left_loop_content, true),
            press_right_loop_content: DrawUnit::new(press_right_loop_content, true),
            dbg_header: DrawUnit::new(dbg_header, true),
            dbg_rx: DrawUnit::new(dbg_rx, true),
            dbg_queue: DrawUnit::new(dbg_queue, true),
            clk_header: DrawUnit::new(clk_header, true),
            clk_freq: DrawUnit::new(clk_freq, true),
            layer_header: DrawUnit::new(layer_header, true),
            perm_layer: DrawUnit::new(static_draw_unit_string!("..."), true),
            underscores_need_redraw: true,
        }
    }

    pub fn hide(&mut self) {
        self.handle.clear();
        self.hidden = true;
    }

    pub fn show(&mut self) {
        if self.hidden {
            self.header.needs_redraw = true;
            self.scan_loop_header.needs_redraw = true;
            self.scan_loop_content.needs_redraw = true;
            self.press_left_loop_content.needs_redraw = true;
            self.press_right_loop_content.needs_redraw = true;
            self.dbg_header.needs_redraw = true;
            self.dbg_rx.needs_redraw = true;
            self.dbg_queue.needs_redraw = true;
            self.clk_header.needs_redraw = true;
            self.clk_freq.needs_redraw = true;
            self.layer_header.needs_redraw = true;
            self.perm_layer.needs_redraw = true;
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

    pub fn update_left_counter(&mut self, avg_latency: f32) {
        self.press_left_loop_content.content.clear();
        let _ = self
            .press_left_loop_content
            .content
            .write_fmt(format_args!("L {avg_latency:.1}"));
        self.press_left_loop_content.needs_redraw = true;
    }

    pub fn update_right_counter(&mut self, avg_latency: f32) {
        self.press_right_loop_content.content.clear();
        let _ = self
            .press_right_loop_content
            .content
            .write_fmt(format_args!("R {avg_latency:.1}"));
        self.press_right_loop_content.needs_redraw = true;
    }

    pub fn update_layer(&mut self, default_layer: OledLineString) {
        self.perm_layer.content = default_layer;
        self.perm_layer.needs_redraw = true;
    }

    pub fn update_rx(&mut self, count: u16) {
        self.dbg_rx.content.clear();
        let _ = self.dbg_rx.content.write_fmt(format_args!("R {count}"));
        self.dbg_rx.needs_redraw = true;
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
        if self.press_left_loop_content.needs_redraw {
            let _ = self.handle.clear_line(28);
            let _ = self
                .handle
                .write_header(28, self.press_left_loop_content.content.as_str());
            self.press_left_loop_content.needs_redraw = false;
        }
        if self.press_right_loop_content.needs_redraw {
            let _ = self.handle.clear_line(36);
            let _ = self
                .handle
                .write_header(36, self.press_right_loop_content.content.as_str());
            self.press_right_loop_content.needs_redraw = false;
        }
        if self.dbg_header.needs_redraw {
            let _ = self.handle.clear_line(48);
            let _ = self
                .handle
                .write_header(48, self.dbg_header.content.as_str());
            self.dbg_header.needs_redraw = false;
        }
        if self.dbg_rx.needs_redraw {
            let _ = self.handle.clear_line(56);
            let _ = self.handle.write_header(56, self.dbg_rx.content.as_str());
            self.dbg_rx.needs_redraw = false;
        }
        if self.dbg_queue.needs_redraw {
            let _ = self.handle.clear_line(64);
            let _ = self
                .handle
                .write_header(64, self.dbg_queue.content.as_str());
            self.dbg_queue.needs_redraw = false;
        }
        if self.clk_header.needs_redraw {
            let _ = self.handle.clear_line(76);
            let _ = self
                .handle
                .write_header(76, self.clk_header.content.as_str());
            self.clk_header.needs_redraw = false;
        }
        if self.clk_freq.needs_redraw {
            let _ = self.handle.clear_line(84);
            let _ = self.handle.write_header(84, self.clk_freq.content.as_str());
            self.clk_freq.needs_redraw = false;
        }
        if self.layer_header.needs_redraw {
            let _ = self.handle.clear_line(96);
            let _ = self
                .handle
                .write_header(96, self.layer_header.content.as_str());
            self.layer_header.needs_redraw = false;
        }
        if self.perm_layer.needs_redraw {
            let _ = self.handle.clear_line(104);
            let _ = self
                .handle
                .write_header(104, self.perm_layer.content.as_str());
            self.perm_layer.needs_redraw = false;
        }
        if self.underscores_need_redraw {
            // Header
            let _ = self.handle.write_underscored_at(8);
            // Perf
            let _ = self.handle.write_underscored_at(44);
            // Clock
            let _ = self.handle.write_underscored_at(72);
            // Dbg
            let _ = self.handle.write_underscored_at(92);
            // Layer
            let _ = self.handle.write_underscored_at(112);
            self.underscores_need_redraw = false;
        }
    }

    pub fn render_boot_msg(&mut self) {
        self.handle.clear();
        let _ = self.handle.write_header(0, "LEFT");
        let _ = self.handle.write_header(9, "SIDE");
        let _ = self.handle.write_header(18, "ENTER");
        let _ = self.handle.write_header(27, "USB");
        let _ = self.handle.write_header(36, "BOOT");
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
