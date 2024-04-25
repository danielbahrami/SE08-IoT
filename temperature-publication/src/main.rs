use std::thread;
use std::time::Duration;

use esp_idf_hal::adc::config::Config;
use esp_idf_hal::adc::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_sys::{self as _};

const V_MAX: u32 = 2450;
const D_MAX: u32 = 4095;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    let peripherals = Peripherals::take()?;

    let mut adc = AdcDriver::new(peripherals.adc2, &Config::new().calibration(true))?;

    let mut adc_pin: esp_idf_hal::adc::AdcChannelDriver<{ attenuation::DB_11 }, _> =
    AdcChannelDriver::new(peripherals.pins.gpio12)?;

    loop {
        thread::sleep(Duration::from_millis(1000));
        println!("ADC value: {}", adc.read(&mut adc_pin)?);
    }
}
