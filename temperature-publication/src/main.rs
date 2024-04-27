use std::thread;
use std::time::Duration;

use esp_idf_hal::adc::config::Config;
use esp_idf_hal::adc::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, mqtt::client::{EspMqttClient, EspMqttConnection, MqttClientConfiguration, QoS}, nvs::EspDefaultNvsPartition, wifi::{BlockingWifi, EspWifi}};
use esp_idf_sys::{self as _, EspError};
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};

const V_MAX: u32 = 2450;
const D_MAX: u32 = 4095;
const MV: f32 = 10.9;
const WIFI_SSID: &str = "";
const WIFI_PASSWORD: &str = "";
const MQTT_BROKER: &str = "";
const MQTT_CLIENT_ID: &str = "";
const MQTT_COMMAND_TOPIC: &str = "";
const MQTT_RESPONSE_TOPIC: &str = "";
const MQTT_TEST_TOPIC: &str = "";

fn calculate_v_out(d_out: f32, v_max: f32, d_max: f32) -> f32 {
   d_out * v_max / d_max
}

fn calculate_temperature(v_out: f32, mv: f32) -> f32 {
    v_out / mv
}

fn connect_wifi(wifi: &mut BlockingWifi<EspWifi<'static>>) -> anyhow::Result<()> {
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.try_into().unwrap(),
        bssid: None,
        auth_method: AuthMethod::WPA2Personal,
        password: WIFI_PASSWORD.try_into().unwrap(),
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

fn mqtt_create(url: &str, client_id: &str) -> Result<(EspMqttClient<'static>, EspMqttConnection), EspError> {
    let (mqtt_client, mqtt_conn) = EspMqttClient::new(url, &MqttClientConfiguration {
        client_id: Some(client_id), ..Default::default()
    },)?;
    Ok((mqtt_client, mqtt_conn))
}

fn mqtt_run(client: &mut EspMqttClient<'_>, connection: &mut EspMqttConnection, topic: &str) -> Result<(), EspError> {
    std::thread::scope(|s| {
        print!("Starting MQTT client");
        std::thread::Builder::new().stack_size(6000).spawn_scoped(s, move || {
            println!("MQTT listening for messages");
            while let Ok(event) = connection.next() {
                println!("[Queue] Event: {}", event.payload());
            }
            println!("Connection closed");
        }).unwrap();
        client.subscribe(topic, QoS::AtMostOnce)?;
        println!("Subscribed to topic \"{topic}\"");
        std::thread::sleep(Duration::from_millis(500));
        let payload = "Payload test";
        loop {
            client.enqueue(topic, QoS::AtMostOnce, false, payload.as_bytes())?;
            println!("Published \"{payload}\" to topic \"{topic}\"");
            let sleep_secs = 2;
            println!("Now sleeping for {sleep_secs}s...");
            std::thread::sleep(Duration::from_secs(sleep_secs));
        }
    })
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    let peripherals = Peripherals::take()?;

    let mut adc = AdcDriver::new(peripherals.adc1, &Config::new().calibration(true))?;

    let mut adc_pin: esp_idf_hal::adc::AdcChannelDriver<{ attenuation::DB_11 }, _> =
    AdcChannelDriver::new(peripherals.pins.gpio34)?;

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let mut wifi = BlockingWifi::wrap(EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?, sys_loop,)?;
    connect_wifi(&mut wifi)?;
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    println!("Wifi DHCP info: {:?}", ip_info);

    let (mut client, mut conn) = mqtt_create(MQTT_BROKER, MQTT_CLIENT_ID).unwrap();
    mqtt_run(&mut client, &mut conn, MQTT_TEST_TOPIC).unwrap();

    loop {
        let d_out = adc.read(&mut adc_pin)?;
        println!("Temperature: {:.2} °C", calculate_temperature(calculate_v_out(d_out as f32, V_MAX as f32, D_MAX as f32), MV));
        thread::sleep(Duration::from_millis(1000));
    }
}
