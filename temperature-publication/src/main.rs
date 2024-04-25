use std::thread;
use std::time::Duration;

use esp_idf_hal::adc::config::Config;
use esp_idf_hal::adc::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::{BlockingWifi, EspWifi}};
use esp_idf_sys::{self as _};
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};

const V_MAX: u32 = 2450;
const D_MAX: u32 = 4095;
const SSID: &str = "";
const PASSWORD: &str = "";

fn calculate_v_out(d_out: f32, v_max: f32, d_max: f32) -> f32 {
   d_out * (v_max / d_max)
}

fn calculate_temperature() {

}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: SSID.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: PASSWORD.try_into().unwrap(),
        channel: None,
        ..Default::default()
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    println!("Wifi started");

    wifi.connect()?;
    println!("Wifi connected");

    wifi.wait_netif_up()?;
    println!("Wifi netif up");

    Ok(())
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    let peripherals = Peripherals::take()?;

    let mut adc = AdcDriver::new(peripherals.adc2, &Config::new().calibration(true))?;

    let mut adc_pin: esp_idf_hal::adc::AdcChannelDriver<{ attenuation::DB_11 }, _> =
    AdcChannelDriver::new(peripherals.pins.gpio12)?;

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let mut wifi = BlockingWifi::wrap(EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?, sys_loop,)?;
    connect_wifi(&mut wifi)?;
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    println!("Wifi DHCP info: {:?}", ip_info);

    loop {
        let d_out = adc.read(&mut adc_pin)?;
        println!("{}", calculate_v_out(d_out as f32, V_MAX as f32, D_MAX as f32));
        thread::sleep(Duration::from_millis(1000));
    }
}
