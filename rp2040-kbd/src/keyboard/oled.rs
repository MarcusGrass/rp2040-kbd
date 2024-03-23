use elite_pi::pac::{I2C1, UART0};
use embedded_graphics::Drawable;
use embedded_graphics::geometry::Point;
use embedded_graphics::mono_font::iso_8859_2::FONT_6X9;
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::text::{Baseline, Text};
use rp2040_hal::gpio::bank0::{Gpio0, Gpio1, Gpio2, Gpio3};
use rp2040_hal::gpio::{FunctionI2c, FunctionUart, Pin, PullDown};
use rp2040_hal::uart::{Enabled, UartPeripheral};
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::prelude::{DisplaySize128x32, I2CInterface};
use ssd1306::Ssd1306;

pub struct OledHandle {
    display: Ssd1306<I2CInterface<rp2040_hal::I2C<I2C1, (rp2040_hal::gpio::Pin<Gpio2, FunctionI2c, PullDown>, rp2040_hal::gpio::Pin<Gpio3, FunctionI2c, PullDown>)>>, DisplaySize128x32, BufferedGraphicsMode<DisplaySize128x32>>
}

impl OledHandle {
    pub fn new(display: Ssd1306<I2CInterface<rp2040_hal::I2C<I2C1, (rp2040_hal::gpio::Pin<Gpio2, FunctionI2c, PullDown>, rp2040_hal::gpio::Pin<Gpio3, FunctionI2c, PullDown>)>>, DisplaySize128x32, BufferedGraphicsMode<DisplaySize128x32>>) -> Self {

        Self { display }
    }

    pub fn clear(&mut self) {
        self.display.clear();
    }

    pub fn write(&mut self, l:i32, s: &str) -> bool {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X9)
            .text_color(BinaryColor::On)
            .build();
        if Text::with_baseline(s, Point::new(0, l), text_style, Baseline::Top)
            .draw(&mut self.display).is_ok() {
            return self.display.flush().is_ok();
        }
        false
    }
}
