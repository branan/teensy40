extern "C" {
    fn main();
    static _bss_start: u8;
    static _bss_end: u8;
}

#[link_section = ".startup"]
#[no_mangle]
pub extern "C" fn startup() {
    init_bss();
    unsafe { main() }
}

#[link_section = ".startup"]
fn init_bss() {
    unsafe {
        let mut ptr = &_bss_start as *const u8 as *mut u8;
        let end = &_bss_end as *const u8 as  *mut u8;
        while ptr != end {
            core::ptr::write_volatile(ptr, 0);
            ptr = (ptr as usize +1) as *mut u8;
        }
    }
}
