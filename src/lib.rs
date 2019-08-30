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
//!    iomuxc::Iomuxc
//!    iomuxc::pin
//!    lpuart::LpUart6
//!    debug,
//! };
//! use core::fmt::Write;
//!
//! unsafe fn sleep() -> ! {
//!     loop {
//!         asm!("wfi" :::: "volatile");
//!     }
//! }
//!
//! #[no_mangle]
//! pub extern "C" fn main() {
//!     let mut ccm = Ccm::new();
//!
//!     let mut uart_clock = ccm.uart_clock_selector_mut().unwrap();
//!     uart_clock.set_input(ccm::UartClockInput::Oscillator);
//!     uart_clock.set_divisor(1);
//!
//!     let iomux = ccm.enable::<iomuxc::Iomuxc>().unwrap();
//!     let tx_pin = iomux
//!         .get_pin::<iomuxc::pin::GpioAdB0_02>()
//!         .unwrap()
//!         .into_lpuart_tx();
//!
//!     let mut uart = ccm.enable::<lpuart::LpUart6<(), ()>>().unwrap();
//!     uart.set_clocks(250, 10);
//!     let mut uart = uart.set_tx(tx_pin).0;
//!
//!     writeln!(&mut uart, "hello").unwrap();
//!
//!     unsafe {
//!         sleep();
//!     }
//! }
//!
//! #[panic_handler]
//! fn teensy_panic(_: &core::panic::PanicInfo) -> ! {
//!     unsafe {
//!         debug::enable();
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
