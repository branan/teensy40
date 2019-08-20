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

pub struct LpUart {
    regs: &'static mut LpUartRegs,
}

impl LpUart {
    /// Grab a UART.
    ///
    /// # Safety
    /// This does not check clock gating, pin configuration, or
    /// whether the UART is already in use.
    pub unsafe fn new(id: usize) -> LpUart {
        let addr = 0x4018_4000 + 0x4000 * (id - 1);
        let regs = &mut *(addr as *mut LpUartRegs);
        LpUart { regs }
    }

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

impl core::fmt::Write for LpUart {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            self.send(b);
        }
        Ok(())
    }
}
