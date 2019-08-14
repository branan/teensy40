#![no_builtins]
#![no_main]
#![no_std]
#![feature(asm, const_transmute)]

extern crate teensy40;
use teensy40::debug;

#[no_mangle]
pub extern "C" fn main() {
    unsafe { debug::enable() }
    unsafe { debug::pin(12) }

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
        debug::pin(13);
        loop {
            asm!("wfi" : : : : "volatile");
        }
    }
}
