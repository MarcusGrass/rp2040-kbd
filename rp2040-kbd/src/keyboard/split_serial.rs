use liatris::pac::{PIO0, RESETS};
use rp2040_hal::fugit;
use rp2040_hal::gpio::bank0::Gpio1;
use rp2040_hal::gpio::{FunctionNull, Pin, PullDown};
use rp2040_hal::pio::{PIOExt, Running, UninitStateMachine, SM0, SM1, SM2, SM3};

#[cfg(feature = "left")]
pub struct UartLeft {
    pub(crate) inner: pio_uart::PioUartRx<Gpio1, PIO0, SM0, Running>,
    _prog: pio_uart::RxProgram<PIO0>,
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
        pio: PIO0,
        resets: &mut RESETS,
    ) -> Self {
        let rx_pin = pin.reconfigure();
        let (mut pio, sm0, sm1, sm2, sm3) = pio.split(resets);
        let mut rx_program = pio_uart::install_rx_program(&mut pio).ok().unwrap(); // Should never fail, because no program was loaded yet
        let rx = pio_uart::PioUartRx::new(rx_pin, sm0, &mut rx_program, baud, system_freq).enable();
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
    pub(crate) inner: pio_uart::PioUartTx<Gpio1, PIO0, SM0, Running>,
    _prog: pio_uart::TxProgram<PIO0>,
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
        pio: PIO0,
        resets: &mut RESETS,
    ) -> Self {
        let rx_pin = pin.reconfigure();
        let (mut pio, sm0, sm1, sm2, sm3) = pio.split(resets);
        let mut tx_program = pio_uart::install_tx_program(&mut pio).ok().unwrap(); // Should never fail, because no program was loaded yet
        let rx = pio_uart::PioUartTx::new(rx_pin, sm0, &mut tx_program, baud, system_freq).enable();
        Self {
            inner: rx,
            _prog: tx_program,
            _sm1: sm1,
            _sm2: sm2,
            _sm3: sm3,
        }
    }
}
