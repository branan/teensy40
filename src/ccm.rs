//! Clock Controller Module
//!
//! The CCM (Clock Controller Module) manages the 7 PLLs, as well as
//! various clock selection muxes and all the individual device clock
//! gates.

use bit_field::BitField;
use core::{
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};
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

// TODO: This should be broken out into a common way to access
// registers like this across different hardware modules.
struct SegmentedRegister {
    val: Volatile<u32>,
    set: Volatile<u32>,
    clear: Volatile<u32>,
    _toggle: Volatile<u32>,
}

#[repr(C, packed)]
struct CcmAnalogRegs {
    pll_arm: SegmentedRegister,
    pll_usb1: SegmentedRegister,
    pll_usb2: SegmentedRegister,
    pll_sys: SegmentedRegister,
    pll_sys_ss: Volatile<u32>,
    _pad0: [u32; 3],
    pll_sys_num: Volatile<u32>,
    _pad1: [u32; 3],
    pll_sys_denom: Volatile<u32>,
    _pad2: [u32; 3],
    pll_audio: SegmentedRegister,
    pll_audio_num: Volatile<u32>,
    _pad3: [u32; 3],
    pll_audio_denom: Volatile<u32>,
    _pad4: [u32; 3],
    pll_video: SegmentedRegister,
    pll_video_num: Volatile<u32>,
    _pad5: [u32; 3],
    pll_video_denom: Volatile<u32>,
    _pad6: [u32; 7],
    pll_enet: SegmentedRegister,
    pfd_480: SegmentedRegister,
    pfd_528: SegmentedRegister,
    _pad7: [u32; 16],
    misc0: SegmentedRegister,
    misc1: SegmentedRegister,
    misc2: SegmentedRegister,
}

/// The ARM PLL (PLL1)
///
/// This PLL can only be used as the clock source for the ARM core and
/// adjacent peripherals. It is typically used as the source for
/// `AHB_CLK_ROOT`, `IPG_CLK_ROOT`, and `PERCLK_CLK_ROOT`.
pub struct ArmPll<CCM> {
    ccm: CCM,
}

/// The First USB PLL (PLL3)
///
/// This PLL is used as the clock source for the first USB phy, as
/// well as being an optional clock reference for many peripherals.
pub struct Usb1Pll<CCM> {
    ccm: CCM,
}

/// The `PRE_PERIHP_CLK_SEL` clock mux
///
/// This mux selects one of four clocks to be fed into the glitchless
/// [`PERIPH_CLK_SEL` mux](PeriphClockSelector) for the ARM core
/// clocks. See [the associated enum](PrePeriphClockInput) for details
/// on the possible clock sources.
pub struct PrePeriphClockSelector<CCM> {
    ccm: CCM,
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
pub struct PeriphClock2Selector<CCM> {
    ccm: CCM,
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
pub struct PeriphClockSelector<CCM> {
    ccm: CCM,
}

/// The 'UART_CLK_SEL` clock mux.
///
/// This mux selects the output of either the USB PLL 24MHz oscillator
/// as the clock source for the UARTs. See [the associated
/// enum](UartClockInput) for details on the possible clock sources.
pub struct UartClockSelector<CCM> {
    ccm: CCM,
}

/// The Clock Controller Module
///
/// This struct provides access to the various clocking components of
/// the system. See the [module level documentation](index.html)  for details.
pub struct Ccm {
    regs: &'static mut CcmRegs,
    analog: &'static mut CcmAnalogRegs,
}

/// Indicates an error occured while trying to retrieve a clocking
/// subsystem
#[derive(Debug)]
pub enum ClockError {
    /// Indicates that the clock component is in use, and thus cannot
    /// be modified.
    InUse,
    /// Indicates that the clock gate configuration would lead to a
    /// peripheral being overclocked.
    TooFast,
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

/// The clock input for the [`UART_CLK_SEL` mux](UartClockSelector)
#[derive(PartialEq, Copy, Clone)]
pub enum UartClockInput {
    /// [`Usb1Pll`] divided by six, typically 80MHz
    Usb1PllOverSix,
    /// The 24MHz oscillator
    Oscillator,
}

#[doc(hidden)]
impl From<u32> for UartClockInput {
    fn from(v: u32) -> UartClockInput {
        match v {
            0 => UartClockInput::Usb1PllOverSix,
            1 => UartClockInput::Oscillator,
            _ => panic!("Invalid value for the UartClkSel input"),
        }
    }
}

#[doc(hidden)]
impl From<UartClockInput> for u32 {
    fn from(v: UartClockInput) -> u32 {
        match v {
            UartClockInput::Usb1PllOverSix => 0,
            UartClockInput::Oscillator => 1,
        }
    }
}

/// The various states a device's clock gate can be in
#[derive(PartialEq, Copy, Clone)]
pub enum ClockGate {
    /// The device is always disabled
    Disabled,
    /// The device is always enabled
    Enabled,
    /// The device is only enabled when the package is in the full
    /// awake state, and disabled during low power states.
    EnabledDuringWake,
}

#[doc(hidden)]
impl From<u32> for ClockGate {
    fn from(v: u32) -> ClockGate {
        match v {
            0 => ClockGate::Disabled,
            1 => ClockGate::EnabledDuringWake,
            3 => ClockGate::Enabled,
            _ => panic!("Invalid value for a clock gate"),
        }
    }
}

#[doc(hidden)]
impl From<ClockGate> for u32 {
    fn from(v: ClockGate) -> u32 {
        match v {
            ClockGate::Disabled => 0,
            ClockGate::Enabled => 3,
            ClockGate::EnabledDuringWake => 1,
        }
    }
}

/// The possible multipliers for the core peripheral PLLs
///
/// These are the two multiplier values available for the [`Usb1Pll`],
/// [`Usb2Pll`], and [`SystemPll`]
#[derive(PartialEq, Copy, Clone)]
pub enum PeripheralPllMultiplier {
    Twenty,
    TwentyTwo,
}

#[doc(hidden)]
impl From<u32> for PeripheralPllMultiplier {
    fn from(v: u32) -> PeripheralPllMultiplier {
        match v {
            0 => PeripheralPllMultiplier::Twenty,
            1 => PeripheralPllMultiplier::TwentyTwo,
            _ => panic!("Invalid value for PLL multiplier"),
        }
    }
}

#[doc(hidden)]
impl From<PeripheralPllMultiplier> for u32 {
    fn from(v: PeripheralPllMultiplier) -> u32 {
        match v {
            PeripheralPllMultiplier::Twenty => 0,
            PeripheralPllMultiplier::TwentyTwo => 1,
        }
    }
}

/// Common trait for hardware modules which are clock-gated
///
/// This trait provides the necessary methods for the CCM management
/// code to validate that clocking is correct for a module before
/// handing that module to user code.
///
/// This should never have to be used in user code, and is public only
/// because of rusts privates-in-public rules.
pub trait ClockGated {
    const GATE: (usize, usize);

    /// Query the [`Ccm`] to determine if the clock path to this
    /// device is enabled, and operating at a safe frequency for this
    /// peripheral.
    fn check_clock(ccm: &Ccm) -> Result<(), ClockError>;

    /// Enable this device and return an instance
    ///
    /// # Safety
    /// This must only be called if the device's clock gate is
    /// enabled, and if the clock path leading to that gate is correct
    /// for this peripheral.
    unsafe fn enable() -> Self;

    /// Disable this device, consuming it.
    ///
    /// Calling this directly is not recommended. Instead,
    /// [`Ccm::disable`] should be called so that the associated clock
    /// gate can be cleared.
    fn disable(self);
}

impl<CCM> ArmPll<CCM>
where
    CCM: DerefMut + Deref<Target = Ccm>,
{
    /// Disables this PLL to conserve power
    pub fn disable(&mut self) {
        unsafe {
            // [pll_arm[bypass]
            self.ccm.analog.pll_arm.set.write(1 << 16);
            // pll_arm[enable]
            self.ccm.analog.pll_arm.clear.write(1 << 13);
            // pll_arm[powerdown]
            self.ccm.analog.pll_arm.set.write(1 << 12);
        }
    }
}

impl<CCM> Usb1Pll<CCM>
where
    CCM: Deref<Target = Ccm>,
{
    pub fn multiplier(&self) -> PeripheralPllMultiplier {
        unsafe {
            // pll_usb1[[div_select]
            self.ccm.analog.pll_usb1.val.read().get_bits(1..2).into()
        }
    }

    pub fn enabled(&self) -> bool {
        unsafe {
            // pll_usb1[power] && pll_usb1[enable]
            self.ccm.analog.pll_usb1.val.read().get_bit(12)
                && self.ccm.analog.pll_usb1.val.read().get_bit(13)
        }
    }
}

impl<CCM> PeriphClockSelector<CCM>
where
    CCM: Deref<Target = Ccm>,
{
    /// Query the current clock source used by this mux
    pub fn input(&self) -> PeriphClockInput {
        // cbcdr[periph_clk_sel]
        unsafe { self.ccm.regs.cbcdr.read().get_bits(25..26).into() }
    }
}

impl<CCM> PeriphClockSelector<CCM>
where
    CCM: DerefMut + Deref<Target = Ccm>,
{
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

impl<CCM> PeriphClock2Selector<CCM>
where
    CCM: Deref<Target = Ccm>,
{
    /// Query the current clock source used by this mux
    pub fn input(&self) -> PeriphClock2Input {
        // cbcmr[periph_clk2_sel]
        unsafe { self.ccm.regs.cbcmr.read().get_bits(12..14).into() }
    }
}

impl<CCM> PeriphClock2Selector<CCM>
where
    CCM: DerefMut + Deref<Target = Ccm>,
{
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

impl<CCM> PrePeriphClockSelector<CCM>
where
    CCM: Deref<Target = Ccm>,
{
    /// Query the current clock source used by this mux
    pub fn input(&self) -> PrePeriphClockInput {
        // cbcmr[pre_periph_clk_sel]
        unsafe { self.ccm.regs.cbcmr.read().get_bits(18..20).into() }
    }
}

impl<CCM> PrePeriphClockSelector<CCM>
where
    CCM: DerefMut + Deref<Target = Ccm>,
{
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

impl<CCM> UartClockSelector<CCM>
where
    CCM: Deref<Target = Ccm>,
{
    /// Query the current clock source used by this mux
    pub fn input(&self) -> UartClockInput {
        // cscdr1[uart_clk_sel]
        unsafe { self.ccm.regs.cscdr1.read().get_bits(6..7).into() }
    }

    /// Query the current post-divider for the UART clocks
    pub fn divisor(&self) -> u32 {
        unsafe {
            // cscdr1[uart_clk_podf]
            self.ccm.regs.cscdr1.read().get_bits(0..6) + 1
        }
    }
}

impl<CCM> UartClockSelector<CCM>
where
    CCM: DerefMut + Deref<Target = Ccm>,
{
    /// Set the clock source used by this mux
    pub fn set_input(&mut self, input: UartClockInput) {
        unsafe {
            self.ccm.regs.cscdr1.update(|r| {
                // cscdr1[uart_clk_sel]
                r.set_bits(6..7, input.into());
            });
        }
    }

    /// Set the divisor for the clock used by this mux
    pub fn set_divisor(&mut self, divisor: u32) {
        unsafe {
            self.ccm.regs.cscdr1.update(|r| {
                // cscdr1[uart_clk_podf]
                r.set_bits(0..6, divisor - 1);
            });
        }
    }
}

static CCM_INIT: AtomicBool = AtomicBool::new(false);

impl Ccm {
    /// Grab the CCM
    ///
    /// # Panics
    /// This will panic if there is an outstanding reference to the
    /// CCM.
    pub fn new() -> Ccm {
        let was_init = CCM_INIT.swap(true, Ordering::Acquire);
        if was_init {
            panic!("Cannot initialize CCM: An instance is already outstanding");
        }
        let regs = unsafe { &mut *(0x400F_C000 as *mut CcmRegs) };
        let analog = unsafe { &mut *(0x400D_8000 as *mut CcmAnalogRegs) };
        Ccm { regs, analog }
    }

    /// Enable a [`ClockGated`] hardware module.
    ///
    /// This will force the peripheral to be always on, even when the
    /// package is in sleep mode. Sleeping certain peripherals is not
    /// yet supported.
    pub fn enable<T: ClockGated>(&mut self) -> Result<T, ClockError> {
        unsafe {
            let gate = <T as ClockGated>::GATE;
            if self.clock_gate(gate) != ClockGate::Disabled {
                Err(ClockError::InUse)
            } else {
                self.set_clock_gate(gate, ClockGate::Enabled);
                Ok(<T as ClockGated>::enable())
            }
        }
    }

    /// Disable a [`ClockGated`] hardware module
    pub fn disable<T: ClockGated>(&mut self, instance: T) {
        unsafe {
            let gate = <T as ClockGated>::GATE;
            instance.disable();
            self.set_clock_gate(gate, ClockGate::Disabled);
        }
    }

    /// Get the [ARM PLL](ArmPll) mutably
    ///
    /// # Errors
    /// Returns [`ClockError::InUse`] if a downstream mux is using this clock source.
    pub fn arm_pll_mut(&mut self) -> Result<ArmPll<&mut Self>, ClockError> {
        if self.pre_periph_clock_selector().input() == PrePeriphClockInput::ArmPll
            && self.periph_clock_selector().input() == PeriphClockInput::PrePeriphClock
        {
            Err(ClockError::InUse)
        } else {
            Ok(ArmPll { ccm: self })
        }
    }

    /// Get the [USB1 PLL](Usb1Pll) immutably
    pub fn usb1_pll(&self) -> Usb1Pll<&Self> {
        Usb1Pll { ccm: self }
    }

    /// Get the [`PERIPH_CLK_SEL` mux](PeriphClockSelector) immutably
    pub fn periph_clock_selector(&self) -> PeriphClockSelector<&Self> {
        PeriphClockSelector { ccm: self }
    }

    /// Get the [`PERIPH_CLK_SEL` mux](PeriphClockSelector) mutably
    ///
    /// Since this is a glitchless mux, this method cannot error.
    pub fn periph_clock_selector_mut(&mut self) -> PeriphClockSelector<&mut Self> {
        PeriphClockSelector { ccm: self }
    }

    /// Get the [`PERIPH_CLK2_SEL` mux](PeriphClock2Selector) mutably
    ///
    /// # Errors
    /// Returns [`ClockError::InUse`] if a downstream mux is using this clock source.
    pub fn periph_clock2_selector_mut(
        &mut self,
    ) -> Result<PeriphClock2Selector<&mut Self>, ClockError> {
        if self.periph_clock_selector().input() != PeriphClockInput::PeriphClock2 {
            Ok(PeriphClock2Selector { ccm: self })
        } else {
            Err(ClockError::InUse)
        }
    }

    /// Get the [`PRE_PERIPH_CLK_SEL` mux](PrePeriphClockSelector) immutably
    pub fn pre_periph_clock_selector(&self) -> PrePeriphClockSelector<&Self> {
        PrePeriphClockSelector { ccm: self }
    }

    /// Get the [`PRE_PERIPH_CLK_SEL` mux](PrePeriphClockSelector) mutably
    ///
    /// # Errors
    /// Returns [`ClockError::InUse`] if a downstream mux is using this clock source.
    pub fn pre_periph_clock_selector_mut(
        &mut self,
    ) -> Result<PrePeriphClockSelector<&mut Self>, ClockError> {
        if self.periph_clock_selector().input() != PeriphClockInput::PrePeriphClock {
            Ok(PrePeriphClockSelector { ccm: self })
        } else {
            Err(ClockError::InUse)
        }
    }

    /// Get the [`UART_CLK_SEL` mux](UartClockSelector) immutably
    pub fn uart_clock_selector(&self) -> UartClockSelector<&Self> {
        UartClockSelector { ccm: self }
    }

    /// Get the [`UART_CLK_SEL` mux](UartClockSelector) mutably
    ///
    /// # Errors
    /// Returns [`ClockError::InUse`] if any UART clock gate is enabled.
    pub fn uart_clock_selector_mut(&mut self) -> Result<UartClockSelector<&mut Self>, ClockError> {
        const UART_CLOCK_GATES: [(usize, usize); 8] = [
            (5, 12),
            (0, 14),
            (0, 6),
            (1, 12),
            (3, 1),
            (3, 3),
            (5, 13),
            (6, 7),
        ];

        if UART_CLOCK_GATES
            .iter()
            .copied()
            .map(|gate| self.clock_gate(gate))
            .any(|gate| gate != ClockGate::Disabled)
        {
            Err(ClockError::InUse)
        } else {
            Ok(UartClockSelector { ccm: self })
        }
    }

    /// Query the status of a clock gate
    pub fn clock_gate(&self, gate: (usize, usize)) -> ClockGate {
        let gate_bits = (gate.1 * 2)..(gate.1 * 2 + 2);
        unsafe { self.regs.ccgr[gate.0].read().get_bits(gate_bits).into() }
    }

    /// Toggle the status of a clock gate
    ///
    /// # Safety
    /// * The clock for a device must not be disabled if the device is in use
    /// * The clock gate for a device must only be enabled if the
    ///   clock path leading to the gate is safe for the device.
    pub unsafe fn set_clock_gate(&mut self, gate: (usize, usize), state: ClockGate) {
        let gate_bits = (gate.1 * 2)..(gate.1 * 2 + 2);
        self.regs.ccgr[gate.0].update(|r| {
            r.set_bits(gate_bits, state.into());
        });
    }

    /// Sanitize the clocking environment to bring us to the safest, simplest configuration
    ///
    /// This does a number of things:
    /// * Disables all clock gates that aren't strictly necessary for
    ///   normal usage
    /// * Points remaining clocks at a safe default (typically, the
    ///   24MHz crystal oscillator)
    /// * Disables all PLLs that it can
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
        // The chip documentation claims every clock is enabled at
        // reset. This is true, so far as it goes. However, the boot
        // firmware will disable clocks to *most* of the peripherals,
        // so there are only a few left for us to turn off
        // here.
        // TODO: actually turn off remaining clocks.

        // Swap the secondary core clock mux to the xtal
        self.periph_clock2_selector_mut()
            .unwrap()
            .set_input(PeriphClock2Input::Oscillator);
        super::debug::progress();

        // Move the core clock to the secondary mux
        self.periph_clock_selector_mut()
            .set_input(PeriphClockInput::PeriphClock2);
        super::debug::progress();

        self.arm_pll_mut().unwrap().disable();
        super::debug::progress();
    }
}
