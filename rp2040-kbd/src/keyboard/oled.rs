#[cfg(feature = "left")]
pub mod left;
#[cfg(feature = "right")]
pub mod right;
pub mod section;

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::iso_8859_4::{FONT_4X6, FONT_5X7};
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::{Baseline, Text};
use embedded_graphics::Drawable;
use liatris::pac::I2C1;
use rp2040_hal::gpio::bank0::{Gpio2, Gpio3};
use rp2040_hal::gpio::{FunctionI2c, PullUp};
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::prelude::{Brightness, DisplaySize128x32, I2CInterface};
use ssd1306::Ssd1306;

#[macro_export]
macro_rules! draw_unit_string {
    ($raw: expr) => {{
        let mut s: heapless::String<8> = heapless::String::new();
        let _ = s.push_str($raw);
        s
    }};
}

pub type OledLineString = heapless::String<8>;
pub const OLED_LINE_HEIGHT: u32 = 8;
pub const OLED_LINE_WIDTH: u32 = 32;

pub struct DrawUnit {
    pub content: OledLineString,
    pub needs_redraw: bool,
}

impl DrawUnit {
    pub const fn new(content: OledLineString, needs_redraw: bool) -> Self {
        Self {
            content,
            needs_redraw,
        }
    }
}

pub struct OledHandle {
    display: Ssd1306<
        I2CInterface<
            rp2040_hal::I2C<
                I2C1,
                (
                    rp2040_hal::gpio::Pin<Gpio2, FunctionI2c, PullUp>,
                    rp2040_hal::gpio::Pin<Gpio3, FunctionI2c, PullUp>,
                ),
            >,
        >,
        DisplaySize128x32,
        BufferedGraphicsMode<DisplaySize128x32>,
    >,
}

impl OledHandle {
    pub fn new(
        mut display: Ssd1306<
            I2CInterface<
                rp2040_hal::I2C<
                    I2C1,
                    (
                        rp2040_hal::gpio::Pin<Gpio2, FunctionI2c, PullUp>,
                        rp2040_hal::gpio::Pin<Gpio3, FunctionI2c, PullUp>,
                    ),
                >,
            >,
            DisplaySize128x32,
            BufferedGraphicsMode<DisplaySize128x32>,
        >,
    ) -> Self {
        let _ = display.set_brightness(Brightness::BRIGHTEST);
        Self { display }
    }

    pub fn clear(&mut self) {
        let _ = self.display.clear(BinaryColor::Off);
        let _ = self.display.flush();
    }

    #[inline(never)]
    pub fn write_header(&mut self, l: i32, s: &str) -> bool {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_5X7)
            .text_color(BinaryColor::On)
            .build();
        if Text::with_baseline(s, Point::new(0, l), text_style, Baseline::Top)
            .draw(&mut self.display)
            .is_ok()
        {
            return self.display.flush().is_ok();
        }
        false
    }

    #[inline(never)]
    pub fn write(&mut self, l: i32, s: &str) -> bool {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_4X6)
            .text_color(BinaryColor::On)
            .build();
        if Text::with_baseline(s, Point::new(0, l), text_style, Baseline::Top)
            .draw(&mut self.display)
            .is_ok()
        {
            return self.display.flush().is_ok();
        }
        false
    }

    #[inline(never)]
    pub fn clear_line(&mut self, l: i32) -> bool {
        if self
            .display
            .fill_solid(
                &Rectangle {
                    top_left: Point::new(0, l),
                    size: Size::new(OLED_LINE_WIDTH, OLED_LINE_HEIGHT),
                },
                BinaryColor::Off,
            )
            .is_ok()
        {
            self.display.flush().is_ok()
        } else {
            false
        }
    }

    #[inline(never)]
    pub fn write_underscored_at(&mut self, l: i32) -> bool {
        if self
            .display
            .fill_solid(
                &Rectangle {
                    top_left: Point::new(0, l),
                    size: Size::new(OLED_LINE_WIDTH, 1),
                },
                BinaryColor::On,
            )
            .is_ok()
        {
            self.display.flush().is_ok()
        } else {
            false
        }
    }
    #[inline(never)]
    pub fn write_bad_boot_msg(&mut self) {
        let _ = self.display.clear(BinaryColor::Off);
        let _ = self.display.flush();
        let _ = self.write(0, "BAD");
        let _ = self.write(9, "IMAGE");
        let _ = self.write(18, "FORCE");
        let _ = self.write(27, "BOOT");
        let _ = self.display.flush();
    }
}
