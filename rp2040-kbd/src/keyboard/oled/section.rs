use crate::draw_unit_string;
use crate::keyboard::oled::{DrawUnit, OledHandle, OledLineString};
use core::fmt::{Arguments, Write};
use core::marker::PhantomData;

pub(super) struct OledRender<const N: usize, const M: usize, S> {
    handle: OledHandle,
    hidden: bool,
    sections: heapless::Vec<OledSection<M>, N>,
    underscores_need_redraw: bool,
    _pd: PhantomData<S>,
}

pub(super) trait OledSide {
    const LABEL: &'static str;
}

#[cfg(feature = "left")]
pub(super) struct LeftOledSide;

#[cfg(feature = "left")]
impl OledSide for LeftOledSide {
    const LABEL: &'static str = "LEFT";
}

#[cfg(feature = "right")]
pub(super) struct RightOledSide;

#[cfg(feature = "right")]
impl OledSide for RightOledSide {
    const LABEL: &'static str = "RIGHT";
}

impl<const N: usize, const M: usize, S: OledSide> OledRender<N, M, S> {
    pub(super) fn new(oled: OledHandle, sections: &[heapless::Vec<&'static str, M>; N]) -> Self {
        let mut s = heapless::Vec::new();
        for sect in sections {
            let mut oled_section = OledSection {
                draw_units: heapless::Vec::new(),
            };
            for header in sect {
                let du = DrawUnit::new(draw_unit_string!(header), true);
                let _ = oled_section.draw_units.push(du);
            }
            let _ = s.push(oled_section);
        }
        Self {
            handle: oled,
            hidden: false,
            sections: s,
            underscores_need_redraw: true,
            _pd: PhantomData,
        }
    }

    pub(super) fn fmt_section(&mut self, sect: usize, part: usize, args: Arguments) {
        let Some(s) = self.sections.get_mut(sect) else {
            return;
        };
        let Some(p) = s.draw_units.get_mut(part) else {
            return;
        };
        // Since most often the same thing is redrawn, this saves a lot of writing to the oled
        let mut new = OledLineString::new();
        let _ = new.write_fmt(args);
        if new != p.content {
            p.content = new;
            p.needs_redraw = true;
        }
    }

    #[cfg(feature = "left")]
    pub(super) fn set_section(&mut self, sect: usize, part: usize, val: OledLineString) {
        let Some(s) = self.sections.get_mut(sect) else {
            return;
        };
        let Some(p) = s.draw_units.get_mut(part) else {
            return;
        };
        if p.content != val {
            p.content = val;
            p.needs_redraw = true;
        }
    }

    pub(super) fn hide(&mut self) {
        self.handle.clear();
        self.hidden = true;
    }

    pub(super) fn show(&mut self) {
        if self.hidden {
            for s in &mut self.sections {
                for d in &mut s.draw_units {
                    d.needs_redraw = true;
                }
            }
            self.underscores_need_redraw = true;
            self.hidden = false;
        }
    }

    pub(super) fn render(&mut self) {
        if self.hidden {
            return;
        }
        let mut offset = 0;
        for s in &mut self.sections {
            for d in &mut s.draw_units {
                if d.needs_redraw {
                    let _ = self.handle.clear_line(offset);
                    let _ = self.handle.write_header(offset, d.content.as_str());
                    d.needs_redraw = false;
                }
                offset += 8;
            }
            if self.underscores_need_redraw {
                self.handle.write_underscored_at(offset);
            }
            offset += 4;
        }
        self.underscores_need_redraw = false;
    }

    pub(super) fn render_boot_msg(&mut self) {
        self.handle.clear();
        let _ = self.handle.write_header(0, S::LABEL);
        let _ = self.handle.write_header(9, "SIDE");
        let _ = self.handle.write_header(18, "ENTER");
        let _ = self.handle.write_header(27, "USB");
        let _ = self.handle.write_header(36, "BOOT");
    }
}

struct OledSection<const N: usize> {
    draw_units: heapless::Vec<DrawUnit, N>,
}
