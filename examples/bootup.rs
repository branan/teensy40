#![no_builtins]
#![no_main]
#![no_std]
#![feature(asm)]

extern crate teensy40;
use teensy40::debug;

#[no_mangle]
pub extern "C" fn main() {
    unsafe { debug::enable() }

    let mut ccm = teensy40::ccm::Ccm::new();

    unsafe {
        ccm.sanitize();
    }

    let mut uart_clock = ccm.uart_clock_selector().unwrap();
    uart_clock.set_input(teensy40::ccm::UartClockInput::Oscillator);
    uart_clock.set_divisor(1);

    let mut uart = unsafe {
        // Enable the clock gate for lpuart6 (on Teensy pins 0 and 1)
        ccm.set_clock_gate(3, 3, teensy40::ccm::ClockGate::Enabled);

        // Set the pin for UART TX (alt mode 2 on GPIO_AD_B0_02)
        let reg = 0x401F_80C4 as *mut u32;
        core::ptr::write_volatile(reg, 2);

        let mut uart = teensy40::lpuart::LpUart::new(6);

        // 24MHz / 2500 is a baudrate of 9600, using 10x oversample for recieve
        uart.set_clocks(250, 10);
        uart.enable();
        uart
    };

    use core::fmt::Write;
    writeln!(&mut uart, "hello").unwrap();

    unsafe {
        // Indicate we made it successfully to the end
        debug::pin12();
    }

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
        debug::led();
        loop {
            asm!("wfi" : : : : "volatile");
        }
    }
}
