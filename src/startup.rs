extern "C" {
    fn main();
}

#[link_section = ".startup"]
#[no_mangle]
pub extern "C" fn startup() {
    unsafe { main() }
}
