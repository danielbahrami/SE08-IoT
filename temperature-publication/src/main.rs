use std::thread;
use std::time::Duration;

use esp_idf_hal::adc::config::Config;
use esp_idf_hal::adc::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, mqtt::client::{EspMqttClient, EspMqttConnection, MqttClientConfiguration, QoS}, nvs::EspDefaultNvsPartition, wifi::{BlockingWifi, EspWifi}};
use esp_idf_sys::{self as _, EspError};
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};

const WIFI_SSID: &str = "";
const WIFI_PASSWORD: &str = "";

const MQTT_BROKER: &str = "";
const MQTT_CLIENT_ID: &str = "";
const MQTT_COMMAND_TOPIC: &str = "";
const MQTT_RESPONSE_TOPIC: &str = "";

const T_1: f32 = -50.0;
const T_2: f32 = 50.0;
const V_1: f32 = 2616.0;
const V_2: f32 = 1558.0;
const V_T: f32 = (V_2 - V_1) / (T_2 - T_1);

fn calculate_temperature(mv: f32) -> f32 {
    ((mv - V_1) / V_T) + T_1
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

fn mqtt_run(client: &mut EspMqttClient<'_>, connection: &mut EspMqttConnection, command_topic: &str, response_topic: &str) -> Result<(), EspError> {
    std::thread::scope(|s| {
        print!("Starting MQTT client");
        std::thread::Builder::new().stack_size(6000).spawn_scoped(s, move || {
            println!("MQTT listening for messages");
            while let Ok(event) = connection.next() {
                let payload = event.payload().to_string();
                if payload.starts_with("measure:") {
                    let parts: Vec<&str> = payload.split(',').collect();
                    if parts.len() != 2 {
                        println!("Invalid command payload: {}", payload);
                        continue;
                    }
                    if let (Ok(num_measurements), Ok(interval)) = (parts[0][8..].parse::<u32>(), parts[1].parse::<u64>()) {
                        // execute_measurement(client, response_topic, num_measurements, interval)?;
                    } else {
                        println!("Invalid command payload: {}", payload);
                    }
                }
            }
            println!("Connection closed");
        }).unwrap();
        client.subscribe(command_topic, QoS::AtMostOnce)?;
        println!("Subscribed to topic \"{command_topic}\"");
        std::thread::sleep(Duration::from_millis(500));
        Ok(())
        /* let payload = "Payload test";
        loop {
            client.enqueue(response_topic, QoS::AtMostOnce, false, payload.as_bytes())?;
            println!("Published \"{payload}\" to topic \"{response_topic}\"");
            let sleep_secs = 2;
            println!("Now sleeping for {sleep_secs}s...");
            std::thread::sleep(Duration::from_secs(sleep_secs));
        } */
    })
}

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    let peripherals = Peripherals::take()?;
    let now = std::time::SystemTime::now();

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
    mqtt_run(&mut client, &mut conn, MQTT_COMMAND_TOPIC, MQTT_RESPONSE_TOPIC).unwrap();

    loop {
        let adc_reading = adc.read(&mut adc_pin)?;
        println!("{:.2}", calculate_temperature(adc_reading as f32));
        thread::sleep(Duration::from_millis(1000));
    }
}
