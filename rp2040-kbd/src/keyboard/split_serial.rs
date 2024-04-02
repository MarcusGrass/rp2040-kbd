use embedded_hal::digital::v2::{InputPin, OutputPin, PinState};
use embedded_hal::timer::CountDown;
use embedded_io::{Read, Write};
use liatris::pac::{PIO0, PIO1, RESETS, UART0};
use nb::block;
use pio_uart::{
    install_rx_program, install_tx_program, PioUartRx, PioUartTx, RxProgram, TxProgram,
};
use rp2040_hal::fugit::MicrosDurationU64;
use rp2040_hal::gpio::bank0::{Gpio0, Gpio1, Gpio2};
use rp2040_hal::gpio::{
    FunctionNull, FunctionSio, FunctionUart, Pin, PullBusKeep, PullDown, PullUp, SioInput,
    SioOutput,
};
use rp2040_hal::pio::{PIOExt, Running, UninitStateMachine, SM0, SM1, SM2, SM3};
use rp2040_hal::uart::{Enabled, UartPeripheral};
use rp2040_hal::{fugit, Timer};

#[cfg(feature = "left")]
pub struct UartLeft {
    pub(crate) inner: pio_uart::PioUartRx<Gpio1, PIO0, SM0, Running>,
    _prog: RxProgram<PIO0>,
    _sm1: UninitStateMachine<(PIO0, SM1)>,
    _sm2: UninitStateMachine<(PIO0, SM2)>,
    _sm3: UninitStateMachine<(PIO0, SM3)>,
}

#[cfg(feature = "left")]
impl UartLeft {
    pub fn new(
        pin: Pin<Gpio1, FunctionNull, PullDown>,
        baud: fugit::HertzU32,
        system_freq: fugit::HertzU32,
        mut pio: PIO0,
        resets: &mut RESETS,
    ) -> Self {
        let rx_pin = pin.reconfigure();
        let (mut pio, sm0, sm1, sm2, sm3) = pio.split(resets);
        let mut rx_program = install_rx_program(&mut pio).ok().unwrap(); // Should never fail, because no program was loaded yet
        let rx = PioUartRx::new(rx_pin, sm0, &mut rx_program, baud, system_freq).enable();
        Self {
            inner: rx,
            _prog: rx_program,
            _sm1: sm1,
            _sm2: sm2,
            _sm3: sm3,
        }
    }
}

#[cfg(feature = "right")]
pub struct UartRight {
    pub(crate) inner: PioUartTx<Gpio1, PIO0, SM0, Running>,
    _prog: TxProgram<PIO0>,
    _sm1: UninitStateMachine<(PIO0, SM1)>,
    _sm2: UninitStateMachine<(PIO0, SM2)>,
    _sm3: UninitStateMachine<(PIO0, SM3)>,
}

#[cfg(feature = "right")]
impl UartRight {
    pub fn new(
        pin: Pin<Gpio1, FunctionNull, PullDown>,
        baud: fugit::HertzU32,
        system_freq: fugit::HertzU32,
        mut pio: PIO0,
        resets: &mut RESETS,
    ) -> Self {
        let rx_pin = pin.reconfigure();
        let (mut pio, sm0, sm1, sm2, sm3) = pio.split(resets);
        let mut tx_program = install_tx_program(&mut pio).ok().unwrap(); // Should never fail, because no program was loaded yet
        let rx = PioUartTx::new(rx_pin, sm0, &mut tx_program, baud, system_freq).enable();
        Self {
            inner: rx,
            _prog: tx_program,
            _sm1: sm1,
            _sm2: sm2,
            _sm3: sm3,
        }
    }
}
