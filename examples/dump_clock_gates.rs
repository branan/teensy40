#![no_builtins]
#![no_main]
#![no_std]
#![feature(asm)]

extern crate teensy40;
use teensy40::*;

const CLOCK_DESCRIPTIONS: [[&str; 16]; 8] = [
    [
        "aips_tz1",
        "aips_tz2",
        "mqs_hmclk_clock",
        "*reserved*",
        "sim_m_mainclk_r",
        "dcp",
        "lpuart3",
        "can1",
        "can1_serial",
        "can2",
        "can2_serial",
        "trace",
        "gpt2_bus",
        "gpt2_serial",
        "lpuart2",
        "gpio2",
    ],
    [
        "lpspi1",
        "lpspi2",
        "lpspi3",
        "lpspi4",
        "adc2",
        "enet",
        "pit",
        "aoi2",
        "adc1",
        "semc_exsc",
        "gpt",
        "gpt_serial",
        "lpuart4",
        "gpio1",
        "csu",
        "gpio5",
    ],
    [
        "ocram_exsc",
        "csi",
        "iomuxc_snvs",
        "lpi2c1",
        "lpi2c2",
        "lpi2c3",
        "iim",
        "xbar3",
        "ipmux1",
        "ipmux2",
        "ipmux3",
        "xbar1",
        "xbar2",
        "gpio3",
        "lcd",
        "pxp",
    ],
    [
        "flexio2",
        "lpuart5",
        "semc",
        "lpuart6",
        "aoi1",
        "lcdif_pix",
        "gpio4",
        "ewm",
        "wdog1",
        "flexram",
        "acmp1",
        "acmp2",
        "acmp3",
        "acmp4",
        "ocram",
        "iomuxc_snvs_gpr",
    ],
    [
        "sim_m7_mainclk_r",
        "iomuxc",
        "iomuxc_gpr",
        "bee",
        "sim_m7",
        "tsc",
        "sim_m",
        "sim_ems",
        "pwm1",
        "pwm2",
        "pwm3",
        "pwm4",
        "enc1",
        "enc2",
        "enc3",
        "enc4",
    ],
    [
        "rom", "flexio1", "wdog3", "dma", "kpp", "wdog2", "aips_tz4", "spdif", "sim_main", "sai1",
        "sai2", "sai3", "lpuart1", "lpuart7", "snvs_hp", "snvs_lp",
    ],
    [
        "usboh3",
        "usdhc1",
        "usdhc2",
        "dcdc",
        "ipmux4",
        "flexspi",
        "trng",
        "lpuart8",
        "timer4",
        "aips_tz3",
        "sim_axbs_p",
        "anadig",
        "lpi2c4_serial",
        "timer1",
        "timer2",
        "timer3",
    ],
    [
        "enet2",
        "flexspi2",
        "axbs_l",
        "can3",
        "can3_serial",
        "aips_lite",
        "flexio3_clk",
        "*reserved*",
        "*reserved*",
        "*reserved*",
        "*reserved*",
        "*reserved*",
        "*reserved*",
        "*reserved*",
        "*reserved*",
        "*reserved*",
    ],
];

#[no_mangle]
pub extern "C" fn main() {
    let mut ccm = ccm::Ccm::new();

    let mut uart_clock = ccm.uart_clock_selector_mut().unwrap();
    uart_clock.set_input(ccm::UartClockInput::Oscillator);
    uart_clock.set_divisor(1);

    let iomux = ccm.enable::<iomuxc::Iomuxc>().unwrap();
    let tx_pin = iomux
        .get_pin::<iomuxc::pin::GpioAdB0_02>()
        .unwrap()
        .into_lpuart_tx();

    let mut uart = ccm.enable::<lpuart::LpUart6<(), ()>>().unwrap();
    uart.set_clocks(250, 10);
    let mut uart = uart.set_tx(tx_pin).0;

    for (reg, descriptions) in CLOCK_DESCRIPTIONS.iter().enumerate() {
        for (gate, desc) in descriptions.iter().enumerate() {
            let state = ccm.clock_gate((reg, gate));

            use core::fmt::Write;
            if state != ccm::ClockGate::Disabled {
                writeln!(&mut uart, "({}, {:02}) {}\r", reg, gate, desc).unwrap();
            }
        }
    }

    // Sleep forever
    loop {
        unsafe {
            asm!("wfi" :::: "volatile");
        }
    }
}

#[panic_handler]
fn teensy_panic(_: &core::panic::PanicInfo) -> ! {
    // Enable the pin
    unsafe {
        debug::enable();
        debug::led();
        loop {
            asm!("wfi" :::: "volatile");
        }
    }
}
