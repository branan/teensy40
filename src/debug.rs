/// Enable the GPIO for debug output
///
/// Safety: Must not use GPIO 2 or 7 in code.
/// Safety: Must call before any other debug fn
pub unsafe fn enable() {
    // Switch from GPIO2 to GPIO 7
    let reg = 0x400A_C06C as *mut u32;
    core::ptr::write_volatile(reg, 0xFFFF_FFFF);
    // Set GPIO to output mode
    let reg = 0x4200_4004 as *mut u32;
    core::ptr::write_volatile(reg, 0xFFFF_FFFF);
}

/// Enable a debug pin
///
/// Safety: Must call `enable_debug_gpio` first
pub unsafe fn pin(pin: u32) {
    let reg = 0x4200_4084 as *mut u32;
    if pin == 12 {
        core::ptr::write_volatile(reg, 1 << 1);
    } else if pin == 13 {
        core::ptr::write_volatile(reg, 1 << 3);
    } else {
        // Do nothing
    }
}
