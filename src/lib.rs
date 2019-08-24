//! Teensy 4.0 in Rust
//!
//! This crate provides wrappers for safe (close to) bare-metal access
//! to the i.MX RT1062 chip found in the Teensy 4.0
//!
//! # Examples
//! ```rust
//! #![no_builtins]
//! #![no_main]
//! #![no_std]
//! #![features(asm)]
//!
//! use teensy40::{
//!    ccm::Ccm,
//!    debug,
//! };
//!
//! unsafe fn sleep() -> ! {
//!     loop {
//!         asm!("wfi" :::: "volatile");
//!     }
//! }
//!
//! #[no_mangle]
//! pub extern "C" fn main() {
//!     unsafe { debug::enable() }
//!     let mut ccm = Ccm::new();
//!     unsafe {
//!         ccm.sanitize();
//!         debug::pin12();
//!         sleep();
//!     }
//! }
//!
//! #[panic_handler]
//! fn teensy_panic(_: &core::panic::PanicInfo) -> ! {
//!     unsafe {
//!         debug::led();
//!         sleep();
//!     }
//! }
//! ```

#![no_builtins]
#![no_std]
#![feature(const_transmute)]

mod bootdata;
mod startup;

pub mod ccm;
pub mod debug;
pub mod iomuxc;
pub mod lpuart;
