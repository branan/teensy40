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
    ($name:ident, $gate:expr, $addr:expr) => {
        pub struct $name {
            regs: &'static mut LpUartRegs,
        }

        impl super::ccm::ClockGated for $name {
            const GATE: (usize, usize) = $gate;

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
                        if (ccm.usb1_pll().multiplier() == PeripheralPllMultiplier::Twenty
                            || ccm.uart_clock_selector().divisor() > 1)
                            && ccm.usb1_pll().enabled()
                        {
                            Ok(())
                        } else {
                            Err(ClockError::TooFast)
                        }
                    }
                }
            }

            unsafe fn enable() -> Self {
                let regs = &mut *($addr as *mut LpUartRegs);
                $name { regs }
            }

            fn disable(self) {}
        }

        impl $name {
            /// Set the baud rate
            ///
            /// # Safety
            /// Does not check if the transmitter or reciever are enabled.
            pub unsafe fn set_clocks(&mut self, divisor: u32, oversample: u32) {
                self.regs.baud.update(|r| {
                    // baud[osr]
                    r.set_bits(24..29, oversample - 1);
                    r.set_bits(0..13, divisor);
                });
            }

            /// Enable the transmitter
            ///
            /// # Safety
            /// Does not validate any pin configuration
            pub unsafe fn enable(&mut self) {
                self.regs.ctrl.update(|r| {
                    // ctrl[te]
                    r.set_bit(19, true);
                });
            }

            pub fn send(&mut self, byte: u8) {
                unsafe {
                    self.regs.data.write(u32::from(byte));

                    // state[tc]
                    // TC is set when there is no pending data to be sent.
                    while !self.regs.stat.read().get_bit(22) {}
                }
            }
        }

        impl core::fmt::Write for $name {
            fn write_str(&mut self, s: &str) -> core::fmt::Result {
                for b in s.bytes() {
                    self.send(b);
                }
                Ok(())
            }
        }
    };
}

uart!(LpUart1, (5, 12), 0x4018_4000);
uart!(LpUart2, (0, 14), 0x4018_8000);
uart!(LpUart3, (0, 6), 0x4018_C000);
uart!(LpUart4, (1, 12), 0x4019_0000);
uart!(LpUart5, (3, 1), 0x4019_4000);
uart!(LpUart6, (3, 3), 0x4019_8000);
uart!(LpUart7, (5, 13), 0x4019_C000);
uart!(LpUart8, (6, 7), 0x401A_0000);
