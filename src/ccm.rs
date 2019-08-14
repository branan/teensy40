use bit_field::BitField;
use core::sync::atomic::{AtomicBool, Ordering};
use volatile::{ReadOnly, Volatile};

#[repr(C, packed)]
struct CcmRegs {
    ccr: Volatile<u32>,
    _pad0: u32,
    csr: ReadOnly<u32>,
    ccsr: Volatile<u32>,
    cacrr: Volatile<u32>,
    cbcdr: Volatile<u32>,
    cbcmr: Volatile<u32>,
    cscmr: [Volatile<u32>; 2],
    cscdr1: Volatile<u32>,
    cs1cdr: Volatile<u32>,
    cs2cdr: Volatile<u32>,
    cdcdr: Volatile<u32>,
    _pad1: u32,
    cscdr2: Volatile<u32>,
    cscdr3: Volatile<u32>,
    _pad2: [u32; 2],
    cdhipr: Volatile<u32>,
    _pad3: [u32; 2],
    clpcr: Volatile<u32>,
    cisr: Volatile<u32>,
    cimr: Volatile<u32>,
    ccosr: Volatile<u32>,
    cgpr: Volatile<u32>,
    ccgr: [Volatile<u32>; 8],
    cmeor: Volatile<u32>,
}

pub struct ArmPll<'a> {
    ccm: &'a mut Ccm,
}

pub struct PrePeriphClockSelector<'a> {
    ccm: &'a mut Ccm,
}

pub struct PeriphClock2Selector<'a> {
    ccm: &'a mut Ccm,
}

pub struct PeriphClockSelector<'a> {
    ccm: &'a mut Ccm,
}

pub struct Ccm {
    regs: &'static mut CcmRegs,
}

/// The state of a clock gate
#[derive(Copy, Clone)]
pub enum ClockState {
    /// The connected clock is always disabled
    Off,
    /// The connected clock is enabled when the package is in `run` mode, but disabled in `wait` or `stop` mode.
    OnWhenAwake,
    /// The connected clock is always enabled
    On,
}

impl core::convert::From<ClockState> for u32 {
    fn from(state: ClockState) -> u32 {
        match state {
            ClockState::Off => 0,
            ClockState::OnWhenAwake => 1,
            ClockState::On => 3,
        }
    }
}

#[derive(Debug)]
pub enum ClockError {
    InUse,
}

#[derive(PartialEq, Copy, Clone)]
pub enum PrePeriphClockInput {
    ArmPll,
    SystemPll,
    SystemPllPfd0,
    SystemPllPfd2,
}

impl From<u32> for PrePeriphClockInput {
    fn from(v: u32) -> PrePeriphClockInput {
        match v {
            0 => PrePeriphClockInput::SystemPll,
            1 => PrePeriphClockInput::SystemPllPfd2,
            2 => PrePeriphClockInput::SystemPllPfd0,
            3 => PrePeriphClockInput::ArmPll,
            _ => panic!("Invalid value for the PrePreiphClkSel input"),
        }
    }
}

impl From<PrePeriphClockInput> for u32 {
    fn from(v: PrePeriphClockInput) -> u32 {
        match v {
            PrePeriphClockInput::ArmPll => 3,
            PrePeriphClockInput::SystemPll => 0,
            PrePeriphClockInput::SystemPllPfd0 => 2,
            PrePeriphClockInput::SystemPllPfd2 => 1,
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum PeriphClock2Input {
    SystemPllBypass,
    Usb1Pll,
    Oscillator,
}

impl From<u32> for PeriphClock2Input {
    fn from(v: u32) -> PeriphClock2Input {
        match v {
            0 => PeriphClock2Input::Usb1Pll,
            1 => PeriphClock2Input::Oscillator,
            2 => PeriphClock2Input::SystemPllBypass,
            _ => panic!("Invalid value for the PeriphClock2Sel input"),
        }
    }
}

impl From<PeriphClock2Input> for u32 {
    fn from(v: PeriphClock2Input) -> u32 {
        match v {
            PeriphClock2Input::SystemPllBypass => 2,
            PeriphClock2Input::Usb1Pll => 0,
            PeriphClock2Input::Oscillator => 1,
        }
    }
}

#[derive(PartialEq, Copy, Clone)]
pub enum PeriphClockInput {
    PrePeriphClock,
    PeriphClock2,
}

impl From<u32> for PeriphClockInput {
    fn from(v: u32) -> PeriphClockInput {
        match v {
            0 => PeriphClockInput::PrePeriphClock,
            1 => PeriphClockInput::PeriphClock2,
            _ => panic!("Invalid value for the PeriphClkSel input"),
        }
    }
}

impl From<PeriphClockInput> for u32 {
    fn from(v: PeriphClockInput) -> u32 {
        match v {
            PeriphClockInput::PrePeriphClock => 0,
            PeriphClockInput::PeriphClock2 => 1,
        }
    }
}

impl PeriphClockSelector<'_> {
    pub fn input(&self) -> PeriphClockInput {
        // cbcdr[periph_clk_sel]
        unsafe { self.ccm.regs.cbcdr.read().get_bits(25..26).into() }
    }

    pub fn set_input(&mut self, input: PeriphClockInput) {
        unsafe {
            self.ccm.regs.cbcdr.update(|r| {
                // cbcdr[periph_clk_sel]
                r.set_bits(25..26, input.into());
            });

            // Once we've set the clock input, we need to wait for the
            // transfer to complete.

            // cdhipr[periph_clk_sel_busy]
            while self.ccm.regs.cdhipr.read().get_bit(5) {}
        }
    }
}

impl PeriphClock2Selector<'_> {
    pub fn input(&self) -> PeriphClock2Input {
        // cbcmr[periph_clk2_sel]
        unsafe { self.ccm.regs.cbcmr.read().get_bits(12..14).into() }
    }

    pub fn set_input(&mut self, input: PeriphClock2Input) {
        unsafe {
            self.ccm.regs.cbcmr.update(|r| {
                // cbcmr[periph_clk2_sel]
                r.set_bits(12..14, input.into());
            });

            // Once we've set the clock input, we need to wait for the
            // transfer to complete.

            // cdhipr[periph2_clk_sel_busy]
            while self.ccm.regs.cdhipr.read().get_bit(3) {}
        }
    }
}

impl PrePeriphClockSelector<'_> {
    pub fn input(&self) -> PrePeriphClockInput {
        // cbcmr[pre_periph_clk_sel]
        unsafe { self.ccm.regs.cbcmr.read().get_bits(18..20).into() }
    }

    pub fn set_input(&mut self, input: PrePeriphClockInput) {
        unsafe {
            self.ccm.regs.cbcmr.update(|r| {
                // cbcmr[pre_periph_clk_sel]
                r.set_bits(18..20, input.into());
            });
        }
    }
}

static CCM_INIT: AtomicBool = AtomicBool::new(false);

impl Ccm {
    pub fn new() -> Ccm {
        let was_init = CCM_INIT.swap(true, Ordering::Acquire);
        if was_init {
            panic!("Cannot initialize CCM: An instance is already outstanding");
        }
        let regs = unsafe { &mut *(0x400F_C000 as *mut CcmRegs) };
        Ccm { regs }
    }

    pub fn arm_pll(&mut self) -> Result<ArmPll, ClockError> {
        if self.pre_periph_clock_selector()?.input() == PrePeriphClockInput::ArmPll
            && self.periph_clock_selector().input() == PeriphClockInput::PrePeriphClock
        {
            Err(ClockError::InUse)
        } else {
            Ok(ArmPll { ccm: self })
        }
    }

    pub fn periph_clock_selector(&mut self) -> PeriphClockSelector {
        PeriphClockSelector { ccm: self }
    }

    pub fn periph_clock2_selector(&mut self) -> Result<PeriphClock2Selector, ClockError> {
        if self.periph_clock_selector().input() != PeriphClockInput::PeriphClock2 {
            Ok(PeriphClock2Selector { ccm: self })
        } else {
            Err(ClockError::InUse)
        }
    }

    pub fn pre_periph_clock_selector(&mut self) -> Result<PrePeriphClockSelector, ClockError> {
        if self.periph_clock_selector().input() != PeriphClockInput::PrePeriphClock {
            Ok(PrePeriphClockSelector { ccm: self })
        } else {
            Err(ClockError::InUse)
        }
    }

    /// Sanitize the clocking environment to bring us to the safest, simplest configuration
    ///
    /// This does a number of things:
    /// * Disables all clock gates that aren't strictly necessary for normal usage
    /// * Points remaning clocks at a safe default (typically, the 24MHz crystal oscillator)
    /// * Disables all PLLs
    ///
    /// This is unsafe because it forcibly disables all clock gates.
    ///
    /// This will panic if it cannot gain access to the peripheral
    /// objects it needs to do its job.
    pub unsafe fn sanitize(&mut self) {
        // TODO: Disable as many clock gates as we can here.

        // Swap the secondary core clock mux to the xtal
        self.periph_clock2_selector()
            .unwrap()
            .set_input(PeriphClock2Input::Oscillator);
        super::debug::progress();

        // Move the core clock to the secondary mux
        self.periph_clock_selector()
            .set_input(PeriphClockInput::PeriphClock2);
        super::debug::progress();
    }

    // unsafe fn set_clock_gate(&self, reg: usize, field: usize, state: ClockState) {
    //     let val = state.into();
    //     self.regs.ccgr[reg].update(|r| {
    //         r.set_bits((field*2)..(field*2+2), state.into())
    //     });
    // }
}
