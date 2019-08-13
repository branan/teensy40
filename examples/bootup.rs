#![no_builtins]
#![no_main]
#![no_std]
#![feature(asm, const_transmute)]

extern crate teensy40;

#[no_mangle]
pub extern "C" fn main() {
    // Switch from GPIO2 to GPIO 7
    let reg = 0x400A_C06C as *mut u32;
    unsafe { core::ptr::write_volatile(reg, 0xFFFF_FFFF) };

    // Set GPIO to output mode
    let reg = 0x4200_4004 as *mut u32;
    unsafe { core::ptr::write_volatile(reg, 1 << 3) };

    // Enable the pin
    let reg = 0x4200_4084 as *mut u32;
    unsafe { core::ptr::write_volatile(reg, 1 << 3) };

    // Sleep forever
    loop {
        unsafe {
            asm!("wfi" :::: "volatile");
        }
    }
}

#[panic_handler]
fn teensy_panic(_: &core::panic::PanicInfo) -> ! {
    loop {
        unsafe {
            asm!("wfi" : : : : "volatile");
        }
    }
}
