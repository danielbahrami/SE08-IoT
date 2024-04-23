use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::adc::*;
use esp_idf_hal::adc::config::Config;
use esp_idf_hal::sys::adc_atten_t_ADC_ATTEN_DB_12;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    let peripherals = Peripherals::take().unwrap();

    let mut adc = AdcDriver::new(peripherals.adc2, &Config::new()).unwrap();
    let mut adc_pin: esp_idf_hal::adc::AdcChannelDriver<'_, {adc_atten_t_ADC_ATTEN_DB_12}, Gpio4> = AdcChannelDriver::new(peripherals.pins.gpio4).unwrap();

    loop {
        let sample = adc.read(&mut adc_pin).unwrap();
        println!("{}", sample);
        FreeRtos::delay_ms(1000);
    }
}
