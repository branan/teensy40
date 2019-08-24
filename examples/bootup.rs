#![no_builtins]
#![no_main]
#![no_std]
#![feature(asm)]

extern crate teensy40;
use teensy40::*;

#[no_mangle]
pub extern "C" fn main() {
    let mut ccm = ccm::Ccm::new();

    let mut uart_clock = ccm.uart_clock_selector_mut().unwrap();
    uart_clock.set_input(ccm::UartClockInput::Oscillator);
    uart_clock.set_divisor(1);

    let iomux = ccm.enable::<iomuxc::Iomuxc>().unwrap();
    let tx_pin = iomux
        .get_pin::<iomuxc::pin::GpioAdB0_02>()
        .unwrap()
        .into_lpuart_tx();

    let mut uart = ccm.enable::<lpuart::LpUart6<(), ()>>().unwrap();
    uart.set_clocks(250, 10);
    let mut uart = uart.set_tx(tx_pin).0;

    use core::fmt::Write;
    writeln!(&mut uart, "hello").unwrap();

    // Sleep forever
    loop {
        unsafe {
            asm!("wfi" :::: "volatile");
        }
    }
}

#[panic_handler]
fn teensy_panic(_: &core::panic::PanicInfo) -> ! {
    // Enable the pin
    unsafe {
        debug::enable();
        debug::led();
        loop {
            asm!("wfi" :::: "volatile");
        }
    }
}
