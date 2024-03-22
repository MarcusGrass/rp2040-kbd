use elite_pi::pac::{I2C1, UART0};
use rp2040_hal::gpio::bank0::{Gpio0, Gpio1, Gpio2, Gpio3};
use rp2040_hal::gpio::{FunctionI2c, FunctionUart, Pin, PullDown};
use rp2040_hal::uart::{Enabled, UartPeripheral};
use ssd1306::mode::BufferedGraphicsMode;
use ssd1306::prelude::{DisplaySize128x32, I2CInterface};
use ssd1306::Ssd1306;

pub struct OledHandle {
    oled_i2c: Ssd1306<I2CInterface<rp2040_hal::I2C<I2C1, (rp2040_hal::gpio::Pin<Gpio2, FunctionI2c, PullDown>, rp2040_hal::gpio::Pin<Gpio3, FunctionI2c, PullDown>)>>, DisplaySize128x32, BufferedGraphicsMode<DisplaySize128x32>>
}

impl OledHandle {
    pub fn new(oled_i2c: Ssd1306<I2CInterface<rp2040_hal::I2C<I2C1, (rp2040_hal::gpio::Pin<Gpio2, FunctionI2c, PullDown>, rp2040_hal::gpio::Pin<Gpio3, FunctionI2c, PullDown>)>>, DisplaySize128x32, BufferedGraphicsMode<DisplaySize128x32>>) -> Self {
        /*
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X9)
        .text_color(BinaryColor::On)
        .build();
    Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();
    display.flush().unwrap();

     */
        Self { oled_i2c }
    }
}

