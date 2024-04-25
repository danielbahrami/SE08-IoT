use std::thread;
use std::time::Duration;

use esp_idf_hal::adc::config::Config;
use esp_idf_hal::adc::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_sys::{self as _};

const V_MAX: u32 = 2450;
const D_MAX: u32 = 4095;

fn calculate_v_out(d_out: f32, v_max: f32, d_max: f32) -> f32 {
   d_out * (v_max / d_max)
}

fn calculate_temperature() {

}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    let peripherals = Peripherals::take()?;

    let mut adc = AdcDriver::new(peripherals.adc2, &Config::new().calibration(true))?;

    let mut adc_pin: esp_idf_hal::adc::AdcChannelDriver<{ attenuation::DB_11 }, _> =
    AdcChannelDriver::new(peripherals.pins.gpio12)?;

    loop {
        let d_out = adc.read(&mut adc_pin)?;
        println!("{}", calculate_v_out(d_out as f32, V_MAX as f32, D_MAX as f32));
        thread::sleep(Duration::from_millis(1000));
    }
}
