//! Clock Controller Module
//!
//! The CCM (Clock Controller Module) manages the 7 PLLs, as well as
//! various clock selection muxes and all the individual device clock
//! gates.

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

/// The ARM PLL (PLL1)
///
/// This PLL can only be used as the clock source for the ARM core and
/// adjacent peripherals. It is typically used as the source for
/// `AHB_CLK_ROOT`, `IPG_CLK_ROOT`, and `PERCLK_CLK_ROOT`.
pub struct ArmPll<'a> {
    ccm: &'a mut Ccm,
}

/// The `PRE_PERIHP_CLK_SEL` clock mux
///
/// This mux selects one of four clocks to be fed into the glitchless
/// [`PERIPH_CLK_SEL` mux](PeriphClockSelector) for the ARM core
/// clocks. See [the associated enum](PrePeriphClockInput) for details
/// on the possible clock sources.
pub struct PrePeriphClockSelector<'a> {
    ccm: &'a mut Ccm,
}

/// The `PERIPH_CLK2_SEL` clock mux
///
/// This mux selects one of three clocks to be fed into the glitchless
/// [`PERIPH_CLK_SEL` mux](PeriphClockSelector) for the ARM core
/// clocks. See [the associated enum](PeriphClock2Input) for details
/// on the possible clock sources.
///
/// This should logically be called `PrePeriphClock2Selector`, but is
/// not for consistency with NXP's documentation.
pub struct PeriphClock2Selector<'a> {
    ccm: &'a mut Ccm,
}

/// The `PERIPH_CLK_SEL` clock mux.
///
/// This mux selects the output of either
/// [`PRE_PERIPH_CLK_SEL`](PrePeriphClockSelector) or
/// [`PERIPH_CLK2_SEL`](PeriphClock2Selector) as the source for the
/// ARM core clocks. This is the final mux in the chain for
/// `AHB_CLK_ROOT` and `IPG_CLK_ROOT`, as well as the primary clock
/// source for `PERCLK_CLK_ROOT`. See [the associated
/// enum](PeriphClockInput) for details on the possible clock sources.
///
/// Since the muxes which feed into this one are not glitchless,
/// making any changes to those muxes requires this mux be pointed to
/// the other input. For example, changing the input source of
/// [`PRE_PERIPH_CLK_SEL`](PrePeriphClockSelector) will require this
/// mux be set to [`PeriphClockInput::PeriphClock2`]
///
/// Because this is a glitchless mux, setting its input source does
/// not require disabling downstream consumers.
pub struct PeriphClockSelector<'a> {
    ccm: &'a mut Ccm,
}

/// The Clock Controller Module
///
/// This struct provides access to the various clocking components of
/// the system. See the [module level documentation](index.html)  for details.
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

#[doc(hidden)]
impl core::convert::From<ClockState> for u32 {
    fn from(state: ClockState) -> u32 {
        match state {
            ClockState::Off => 0,
            ClockState::OnWhenAwake => 1,
            ClockState::On => 3,
        }
    }
}

/// Indicates an error occured while trying to retrieve a clocking
/// subsystem
#[derive(Debug)]
pub enum ClockError {
    /// Indicates that the clock component is in use, and thus cannot
    /// be modified.
    InUse,
}

/// The clock source used by the [`PRE_PERIPH_CLK_SEL`
/// mux](PrePeriphClockSelector).
#[derive(PartialEq, Copy, Clone)]
pub enum PrePeriphClockInput {
    /// The [`ArmPll`] output. This PLL can only be accessed through
    /// this clock mux.
    ArmPll,

    /// The [`SystemPll`] output. This Pll is typically also used for
    /// most peripherals on the package.
    SystemPll,

    /// The [`SystemPll`] phased fractional divider output. This
    /// divides the `SystemPll` to a slightly lower frequency.
    SystemPllPfd0,

    /// The [`SystemPll`] phased fractional divider output. This
    /// divides the `SystemPll` to a slightly lower frequency.
    SystemPllPfd2,
}

#[doc(hidden)]
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

#[doc(hidden)]
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

/// The clock input for the [`PERIPH_CLK2_SEL` mux](PeriphClock2Selector).
#[derive(PartialEq, Copy, Clone)]
pub enum PeriphClock2Input {
    /// The [`SystemPll`] bypass source. On a Teensy, this is always
    /// the 24MHz oscillator since the external clock pins are not
    /// used. Choosing this instead of
    /// [`Oscillator`](#variant.Oscillator) below will block the
    /// [`SystemPll`] from being modified.
    SystemPllBypass,

    /// [`Usb1Pll`], the clock for the first USB device.
    Usb1Pll,

    /// The 24MHz oscillator.
    Oscillator,
}

#[doc(hidden)]
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

#[doc(hidden)]
impl From<PeriphClock2Input> for u32 {
    fn from(v: PeriphClock2Input) -> u32 {
        match v {
            PeriphClock2Input::SystemPllBypass => 2,
            PeriphClock2Input::Usb1Pll => 0,
            PeriphClock2Input::Oscillator => 1,
        }
    }
}

/// The clock input for the [`PERIPH_CLK_SEL` mux](PeriphClockSelector).
#[derive(PartialEq, Copy, Clone)]
pub enum PeriphClockInput {
    /// The clock is sourced from [`PRE_PERIPH_CLK_SEL`](PrePeriphClockSelector).
    PrePeriphClock,
    /// The clock is sourced from [`PERIPH_CLK2_SEL`](PeriphClock2Selector).
    PeriphClock2,
}

#[doc(hidden)]
impl From<u32> for PeriphClockInput {
    fn from(v: u32) -> PeriphClockInput {
        match v {
            0 => PeriphClockInput::PrePeriphClock,
            1 => PeriphClockInput::PeriphClock2,
            _ => panic!("Invalid value for the PeriphClkSel input"),
        }
    }
}

#[doc(hidden)]
impl From<PeriphClockInput> for u32 {
    fn from(v: PeriphClockInput) -> u32 {
        match v {
            PeriphClockInput::PrePeriphClock => 0,
            PeriphClockInput::PeriphClock2 => 1,
        }
    }
}

impl PeriphClockSelector<'_> {
    /// Query the current clock source used by this mux
    pub fn input(&self) -> PeriphClockInput {
        // cbcdr[periph_clk_sel]
        unsafe { self.ccm.regs.cbcdr.read().get_bits(25..26).into() }
    }

    /// Set the clock source used for this mux.
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
    /// Query the current clock source used by this mux
    pub fn input(&self) -> PeriphClock2Input {
        // cbcmr[periph_clk2_sel]
        unsafe { self.ccm.regs.cbcmr.read().get_bits(12..14).into() }
    }

    /// Set the clock source used for this mux.
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
    /// Query the current clock source used by this mux
    pub fn input(&self) -> PrePeriphClockInput {
        // cbcmr[pre_periph_clk_sel]
        unsafe { self.ccm.regs.cbcmr.read().get_bits(18..20).into() }
    }

    /// Set the clock source used by this mux
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

    /// Get the [`ArmPll`] for modification.
    ///
    /// # Errors
    /// Returns [`ClockError::InUse`] if a downstream mux is using this clock source.
    pub fn arm_pll(&mut self) -> Result<ArmPll, ClockError> {
        if self.pre_periph_clock_selector()?.input() == PrePeriphClockInput::ArmPll
            && self.periph_clock_selector().input() == PeriphClockInput::PrePeriphClock
        {
            Err(ClockError::InUse)
        } else {
            Ok(ArmPll { ccm: self })
        }
    }

    /// Get the [`PERIPH_CLK_SEL` mux](PeriphClockSelector)
    ///
    /// Since this is a glitchless mux, this method cannot error.
    pub fn periph_clock_selector(&mut self) -> PeriphClockSelector {
        PeriphClockSelector { ccm: self }
    }

    /// Get the [`PERIPH_CLK2_SEL` mux](PeriphClock2Selector)
    ///
    /// # Errors
    /// Returns [`ClockError::InUse`] if a downstream mux is using this clock source.
    pub fn periph_clock2_selector(&mut self) -> Result<PeriphClock2Selector, ClockError> {
        if self.periph_clock_selector().input() != PeriphClockInput::PeriphClock2 {
            Ok(PeriphClock2Selector { ccm: self })
        } else {
            Err(ClockError::InUse)
        }
    }

    /// Get the [`PRE_PERIPH_CLK_SEL` mux](PrePeriphClockSelector)
    ///
    /// # Errors
    /// Returns [`ClockError::InUse`] if a downstream mux is using this clock source.
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
    /// # Safety
    /// This method will forcibly shut down all clock gates, which
    /// renders any outstanding references to hardware modules unsafe
    /// to use. It should only be used early during hardware bringup.
    ///
    /// # Panics
    /// This method will panic if it can't figure out how to disable
    /// all its clocks.
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
}
