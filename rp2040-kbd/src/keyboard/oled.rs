#[cfg(feature = "left")]
pub mod left;
#[cfg(feature = "right")]
pub mod right;

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::iso_8859_2::FONT_6X9;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::Size;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::{Baseline, Text};
use embedded_graphics::Drawable;
use heapless::String;
use liatris::pac::{I2C1, UART0};
use rp2040_hal::gpio::bank0::{Gpio0, Gpio1, Gpio2, Gpio3};
use rp2040_hal::gpio::{FunctionI2c, FunctionUart, Pin, PullDown};
use rp2040_hal::uart::{Enabled, UartPeripheral};
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::prelude::{Brightness, DisplaySize128x32, I2CInterface};
use ssd1306::Ssd1306;

pub trait OledWriter {
    fn write_enter_boot_msg(&mut self);
}

#[inline]
pub fn blank_line() -> String<5> {
    let mut s = String::new();
    let _ = s.push_str("     ");
    s
}

pub struct DrawUnit {
    pub content: String<5>,
    pub needs_redraw: bool,
}

impl DrawUnit {
    #[inline]
    pub fn blank() -> Self {
        Self {
            content: blank_line(),
            needs_redraw: false,
        }
    }
    pub fn new(content: String<5>, needs_redraw: bool) -> Self {
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
                    rp2040_hal::gpio::Pin<Gpio2, FunctionI2c, PullDown>,
                    rp2040_hal::gpio::Pin<Gpio3, FunctionI2c, PullDown>,
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
                        rp2040_hal::gpio::Pin<Gpio2, FunctionI2c, PullDown>,
                        rp2040_hal::gpio::Pin<Gpio3, FunctionI2c, PullDown>,
                    ),
                >,
            >,
            DisplaySize128x32,
            BufferedGraphicsMode<DisplaySize128x32>,
        >,
    ) -> Self {
        display.set_brightness(Brightness::BRIGHTEST);
        Self { display }
    }

    pub fn clear(&mut self) {
        self.display.clear();
        let _ = self.display.flush();
    }

    pub fn write(&mut self, l: i32, s: &str) -> bool {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
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

    pub fn clear_line(&mut self, l: i32) -> bool {
        if self
            .display
            .fill_solid(
                &Rectangle {
                    top_left: Point::new(0, l),
                    size: Size::new(32, 9),
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

    pub fn write_bad_boot_msg(&mut self) {
        let _ = self.display.clear();
        let _ = self.display.flush();
        let _ = self.write(0, "BAD");
        let _ = self.write(9, "IMAGE");
        let _ = self.write(18, "FORCE");
        let _ = self.write(27, "BOOT");
        let _ = self.display.flush();
    }
}
