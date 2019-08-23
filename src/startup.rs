extern "C" {
    fn main();
    static _bss_start: u8;
    static _bss_end: u8;
}

#[link_section = ".startup"]
#[no_mangle]
pub unsafe extern "C" fn startup() {
    init_bss();
    super::ccm::Ccm::new().sanitize();
    main();
}

#[link_section = ".startup"]
unsafe fn init_bss() {
    // This is probably fragile.
    // As far as I can tell, the optimizer is assuming that
    // _bss_start and _bss_end cannot be aliases for the same
    // memory location. This causes a zero-length BSS section to
    // blow up here. re-creating our end pointer with a bit of
    // math seems to trick the optimizer, for now.
    let length = (&_bss_end as *const u8 as usize) - (&_bss_start as *const u8 as usize);
    let mut ptr = &_bss_start as *const u8 as *mut u8;
    let end = (ptr as usize + length) as *const u8;
    while ptr as *const u8 != end {
        core::ptr::write_volatile(ptr, 0);
        ptr = (ptr as usize + 1) as *mut u8;
    }
}
