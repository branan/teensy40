//! Low Power Universal Asynchronous Receiver / Transmitter
//!
//! The `LPUART` modules in the i.MX RT1062 provide the most basic
//! serial data transfer.

use bit_field::BitField;
use volatile::{ReadOnly, Volatile};

#[repr(C, packed)]
struct LpUartRegs {
    verid: ReadOnly<u32>,
    param: ReadOnly<u32>,
    global: Volatile<u32>,
    pincfg: Volatile<u32>,
    baud: Volatile<u32>,
    stat: Volatile<u32>,
    ctrl: Volatile<u32>,
    data: Volatile<u32>,
    r#match: Volatile<u32>,
    modir: Volatile<u32>,
    fifo: Volatile<u32>,
    water: Volatile<u32>,
}

macro_rules! uart {
    ($name:ident, $short_name:ident, $tx_pin:ident, $rx_pin:ident, $gate:expr, $addr:expr) => {
        pub struct $name<T, R> {
            regs: &'static mut LpUartRegs,
            tx: T,
            rx: R,
        }

        pub type $short_name = $name<(), ()>;

        /// This is a marker trait to indicate that a pin can be used
        /// to transmit via this UART
        pub trait $tx_pin {}

        /// This is a marker trait to indicate that a pin can be used
        /// to recieve via this UART.
        pub trait $rx_pin {}

        impl super::ccm::ClockGated for $name<(), ()> {
            const GATE: (usize, usize) = $gate;

            /// For a UART, the final clock frequency must be at most
            /// 80MHz. If the selected clock is the 24MHz oscillator,
            /// this is always true. If it is the USB1 PLL, we must
            /// ensure we have a correct divisor as well.
            ///
            /// To set the clock source and divider, use the
            /// [`UART_CLK_SEL` mux](../ccm/UartClockSelector)
            fn check_clock(ccm: &super::ccm::Ccm) -> Result<(), super::ccm::ClockError> {
                use super::ccm::{ClockError, PeripheralPllMultiplier, UartClockInput};

                match ccm.uart_clock_selector().input() {
                    UartClockInput::Oscillator => Ok(()),
                    UartClockInput::Usb1PllOverSix => {
                        // If we're not using the 24MHz oscillator, we
                        // need to ensure that the final input
                        // frequency for the UARTs is less than
                        // 80MHz. Normally the USB PLL runs at 480MHz,
                        // so we don't need to post-divide it. If it's
                        // been overclocked to 528MHz, we need to
                        // postdivide by at least two.
                        //
                        // We also need to ensure the USB PLL is
                        // enabled if it's our clock source.
                        if !ccm.usb1_pll().enabled() {
                            Err(ClockError::Disabled)
                        } else {
                            if ccm.usb1_pll().multiplier() == PeripheralPllMultiplier::TwentyTwo
                                && ccm.uart_clock_selector().divisor() == 1
                            {
                                Err(ClockError::TooFast)
                            } else {
                                Ok(())
                            }
                        }
                    }
                }
            }

            unsafe fn enable() -> Self {
                let regs = &mut *($addr as *mut LpUartRegs);
                $name {
                    regs,
                    tx: (),
                    rx: (),
                }
            }

            fn disable(self) {}
        }

        impl $name<(), ()> {
            /// Set the baud rate
            ///
            /// This can only be done for a UART which has not had has
            /// a TX or RX pin assigned.
            pub fn set_clocks(&mut self, divisor: u32, oversample: u32) {
                unsafe {
                    self.regs.baud.update(|r| {
                        // baud[osr]
                        r.set_bits(24..29, oversample - 1);
                        r.set_bits(0..13, divisor);
                    });
                }
            }
        }

        impl<T, R> $name<T, R> {
            /// Set the transmit pin
            ///
            /// This updates the typestate of this UART to indicate
            /// that it is enabled for transmit. Being enabled for
            /// transmit allows sending data, and blocks updating the
            /// baud rate.
            pub fn set_tx<Tx>(self, tx: Tx) -> ($name<Tx, R>, T)
            where
                Tx: $tx_pin,
            {
                let regs = self.regs;
                let rx = self.rx;
                let old_tx = self.tx;

                unsafe {
                    regs.ctrl.update(|r| {
                        // ctrl[te]
                        r.set_bit(19, true);
                    });
                }

                ($name { regs, tx, rx }, old_tx)
            }

            /// Set the recieve pin
            ///
            /// This updates the typestate of thsi UART to indicate
            /// that it is enabled for recieve. Recieve is
            /// unimplemented.
            pub fn set_rx<Rx>(self, rx: Rx) -> ($name<T, Rx>, R)
            where
                Rx: $rx_pin,
            {
                let regs = self.regs;
                let tx = self.tx;
                let old_rx = self.rx;
                ($name { regs, tx, rx }, old_rx)
            }
        }

        impl<T, R> $name<T, R>
        where
            T: $tx_pin,
        {
            /// Send a byte of data across this UART
            ///
            /// This can only be done once a transmit pin has been
            /// set. This method will block until the UART has
            /// completed transmission of the byte.
            pub fn send(&mut self, byte: u8) {
                unsafe {
                    self.regs.data.write(u32::from(byte));

                    // state[tc]
                    // TC is set when there is no pending data to be sent.
                    while !self.regs.stat.read().get_bit(22) {}
                }
            }
        }

        impl<T, R> core::fmt::Write for $name<T, R>
        where
            T: $tx_pin,
        {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for b in s.bytes() {
                    self.send(b);
                }
                Ok(())
            }
        }
    };
}

uart!(LpUart1, Uart1, LpUart1Tx, LpUart1Rx, (5, 12), 0x4018_4000);
uart!(LpUart2, Uart2, LpUart2Tx, LpUart2Rx, (0, 14), 0x4018_8000);
uart!(LpUart3, Uart3, LpUart3Tx, LpUart3Rx, (0, 6), 0x4018_C000);
uart!(LpUart4, Uart4, LpUart4Tx, LpUart4Rx, (1, 12), 0x4019_0000);
uart!(LpUart5, Uart5, LpUart5Tx, LpUart5Rx, (3, 1), 0x4019_4000);
uart!(LpUart6, Uart6, LpUart6Tx, LpUart6Rx, (3, 3), 0x4019_8000);
uart!(LpUart7, Uart7, LpUart7Tx, LpUart7Rx, (5, 13), 0x4019_C000);
uart!(LpUart8, Uart8, LpUart8Tx, LpUart8Rx, (6, 7), 0x401A_0000);
