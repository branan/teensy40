//! Debugging helpers using GPIOs
//!
//! These functions allow turning on various pins of the Teensy in
//! order to provide status information when other debugging methods
//! are unavailable. They claim various hardware units without regards
//! to what other code has done, and as such are all unsafe.

use core::sync::atomic::{AtomicU8, Ordering};

/// Enable the GPIO for debug output
///
/// # Safety
/// * This function must be called before any other debug function
/// * GPIO1, GPIO2, GPIO6, and GPIO7 cannot be used once this has been
/// called.
pub unsafe fn enable() {
    // Switch from GPIO1 to GPIO 6
    let reg = 0x400A_C068 as *mut u32;
    core::ptr::write_volatile(reg, 0xFFFF_FFFF);

    // Switch from GPIO2 to GPIO 7
    let reg = 0x400A_C06C as *mut u32;
    core::ptr::write_volatile(reg, 0xFFFF_FFFF);

    // Set GPIO6 to output mode
    let reg = 0x4200_0004 as *mut u32;
    core::ptr::write_volatile(reg, 0xFFFF_FFFF);

    // Set GPIO7 to output mode
    let reg = 0x4200_4004 as *mut u32;
    core::ptr::write_volatile(reg, 0xFFFF_FFFF);
}

unsafe fn pin(pin: u32, reg: *mut u32) {
    core::ptr::write_volatile(reg, 1 << pin);
}

/// Turn on the Teensy's orange LED.
pub unsafe fn led() {
    pin(3, 0x4200_4084 as *mut u32);
}

/// Turn on the Teensy's pin 6
pub unsafe fn pin06() {
    pin(10, 0x4200_4084 as *mut u32);
}

/// Turn on the Teensy's pin 7
pub unsafe fn pin07() {
    pin(17, 0x4200_4084 as *mut u32);
}

/// Turn on the Teensy's pin 8
pub unsafe fn pin08() {
    pin(16, 0x4200_4084 as *mut u32);
}

/// Turn on the Teensy's pin 9
pub unsafe fn pin09() {
    pin(11, 0x4200_4084 as *mut u32);
}

/// Turn on the Teensy's pin 10
pub unsafe fn pin10() {
    pin(0, 0x4200_4084 as *mut u32);
}

/// Turn on the Teensy's pin 11
pub unsafe fn pin11() {
    pin(2, 0x4200_4084 as *mut u32);
}

/// Turn on the Teensy's pin 12
pub unsafe fn pin12() {
    pin(1, 0x4200_4084 as *mut u32);
}

static PROGRESS_COUNTER: AtomicU8 = AtomicU8::new(0);
const PROGRESS_MAX: u8 = 10;

/// Increment the progress bar
///
/// This function increments a 10-segment LED bar graph connected to
/// the Teensy's pins 14 through 23.
///
/// # Safety
/// Must call [`enable`] first
pub unsafe fn progress() {
    if PROGRESS_COUNTER.load(Ordering::Relaxed) >= PROGRESS_MAX {
        return;
    }
    let idx = PROGRESS_COUNTER.fetch_add(1, Ordering::Relaxed);
    if idx >= PROGRESS_MAX {
        return;
    }
    let shift = [18, 19, 23, 22, 17, 16, 26, 27, 24, 25][idx as usize];

    pin(shift, 0x4200_0084 as *mut u32);
}
