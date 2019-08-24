//! The Input/Output Multiplexer Controller
//!
//! The IOMUXC is responsible for mapping pins to individual hardware
//! I/O units.

pub struct Iomuxc {
    _private: (),
}

#[derive(Debug)]
pub enum PinError {
    InUse,
}

impl super::ccm::ClockGated for Iomuxc {
    const GATE: (usize, usize) = (4, 1);

    fn check_clock(_: &super::ccm::Ccm) -> Result<(), super::ccm::ClockError> {
        Ok(())
    }

    unsafe fn enable() -> Self {
        Iomuxc { _private: () }
    }

    fn disable(self) {}
}

impl Iomuxc {
    pub fn get_pin<P: Pin>(&self) -> Result<P, PinError> {
        P::new(self)
    }
}

pub trait Pin: Sized {
    fn new(_: &Iomuxc) -> Result<Self, PinError>;
}

pub mod pin {
    use core::sync::atomic::{AtomicBool, Ordering};

    pub struct GpioAdB0_02 {
        _private: (),
    }

    pub struct GpioAdB0_02LpUartTx {
        _private: (),
    }

    impl GpioAdB0_02 {
        pub fn into_lpuart_tx(self) -> GpioAdB0_02LpUartTx {
            unsafe {
                core::ptr::write_volatile(0x401F_80C4 as *mut u32, 2);
            }
            GpioAdB0_02LpUartTx { _private: () }
        }
    }

    static GPIO_AD_B0_02_INIT: AtomicBool = AtomicBool::new(false);
    impl super::Pin for GpioAdB0_02 {
        fn new(_: &super::Iomuxc) -> Result<Self, super::PinError> {
            let was_init = GPIO_AD_B0_02_INIT.swap(true, Ordering::Acquire);
            if was_init {
                Err(super::PinError::InUse)
            } else {
                Ok(GpioAdB0_02 { _private: () })
            }
        }
    }

    impl super::super::lpuart::LpUart6Tx for GpioAdB0_02LpUartTx {}
}
