use std::sync::mpsc;
use std::time::Duration;
use std::time::SystemTime;

use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_hal::adc::attenuation::DB_11;
use esp_idf_hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_hal::adc::{self};
use esp_idf_hal::gpio::{self};
use esp_idf_hal::peripheral::Peripheral;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    mqtt::client::{EspMqttClient, EspMqttConnection, MqttClientConfiguration, QoS},
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
};
use esp_idf_sys::{self as _, EspError};

const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

const MQTT_BROKER: &str = env!("MQTT_BROKER");
const MQTT_CLIENT_ID: &str = env!("MQTT_CLIENT_ID");
const MQTT_COMMAND_TOPIC: &str = env!("MQTT_COMMAND_TOPIC");
const MQTT_RESPONSE_TOPIC: &str = env!("MQTT_RESPONSE_TOPIC");

const T_1: f32 = -50.0;
const T_2: f32 = 50.0;
const V_1: f32 = 2616.0;
const V_2: f32 = 1558.0;
const V_T: f32 = (V_2 - V_1) / (T_2 - T_1);

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    let peripherals = Peripherals::take()?;
    let uptime = std::time::SystemTime::now();

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;
    connect_wifi(&mut wifi)?;
    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    println!("Wifi DHCP info: {:?}", ip_info);

    let (mut client, conn) = mqtt_create(MQTT_BROKER, MQTT_CLIENT_ID).unwrap();
    mqtt_run(
        &mut client,
        conn,
        MQTT_COMMAND_TOPIC,
        MQTT_RESPONSE_TOPIC,
        peripherals.adc1,
        peripherals.pins.gpio34,
        uptime,
    );

    Ok(())
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

fn mqtt_create(
    url: &str,
    client_id: &str,
) -> Result<(EspMqttClient<'static>, EspMqttConnection), EspError> {
    let (mqtt_client, mqtt_conn) = EspMqttClient::new(
        url,
        &MqttClientConfiguration {
            client_id: Some(client_id),
            ..Default::default()
        },
    )?;
    Ok((mqtt_client, mqtt_conn))
}

fn mqtt_run(
    client: &mut EspMqttClient<'_>,
    mut connection: EspMqttConnection,
    command_topic: &str,
    response_topic: &str,
    adc_driver: adc::ADC1,
    adc_pin: impl Peripheral<P = gpio::Gpio34>,
    uptime: SystemTime,
) {
    let adc_config = AdcChannelConfig {
        attenuation: DB_11,
        calibration: true,
        ..Default::default()
    };
    let adc = AdcDriver::new(adc_driver).unwrap();
    let mut adc_pin = AdcChannelDriver::new(&adc, adc_pin, &adc_config).unwrap();

    let (tx, rx) = mpsc::channel::<String>();

    std::thread::spawn(move || {
        println!("MQTT listening for messages");
        while let Ok(event) = connection.next() {
            let payload = event.payload().to_string();
            if let Some(start_index) = payload.find("measure:") {
                let end_index = payload[start_index..]
                    .find("\"")
                    .map(|i| i + start_index)
                    .unwrap_or(payload.len());
                let payload_data = &payload[start_index..end_index];
                println!("Payload data: {}", payload_data);
                tx.send(payload_data.to_string()).unwrap();
            }
        }
    });
    client.subscribe(command_topic, QoS::AtMostOnce).unwrap();
    println!("Subscribed to topic \"{command_topic}\"");

    loop {
        match rx.recv() {
            Ok(payload) => {
                let parts: Vec<&str> = payload.split(",").collect();
                if parts.len() != 2 {
                    println!("Invalid command: {}", payload);
                    if let Err(err) = client.enqueue(
                        response_topic,
                        QoS::AtMostOnce,
                        false,
                        format!("Invalid command: {}", payload).as_bytes(),
                    ) {
                        eprintln!("Error publishing payload: {:?}", err);
                        break;
                    }
                    continue;
                }
                let num_measurements = match parts[0][8..].parse::<u32>() {
                    Ok(num) => num,
                    Err(_) => {
                        println!("Invalid number of measurements: {}", &parts[0][8..]);
                        if let Err(err) = client.enqueue(
                            response_topic,
                            QoS::AtMostOnce,
                            false,
                            format!("Invalid number of measurements: {}", &parts[0][8..])
                                .as_bytes(),
                        ) {
                            eprintln!("Error publishing payload: {:?}", err);
                            break;
                        }
                        continue;
                    }
                };
                let interval = match parts[1].parse::<u64>() {
                    Ok(interval) => interval,
                    Err(_) => {
                        println!("Invalid interval: {}", parts[1]);
                        if let Err(err) = client.enqueue(
                            response_topic,
                            QoS::AtMostOnce,
                            false,
                            format!("Invalid interval: {}", parts[1]).as_bytes(),
                        ) {
                            eprintln!("Error publishing payload: {:?}", err);
                            break;
                        }
                        continue;
                    }
                };

                for i in (0..num_measurements).rev() {
                    let remaining_messages = i;
                    let temperature = calculate_temperature(adc.read(&mut adc_pin).unwrap() as f32);
                    let uptime = uptime.elapsed().unwrap().as_millis();
                    let response_payload =
                        format!("{},{:.1},{}", remaining_messages, temperature, uptime);
                    if let Err(err) = client.enqueue(
                        response_topic,
                        QoS::AtMostOnce,
                        false,
                        response_payload.as_bytes(),
                    ) {
                        eprintln!("Error publishing response payload: {:?}", err);
                        break;
                    }
                    println!(
                        "Published \"{}\" to topic \"{}\"",
                        response_payload, response_topic
                    );
                    if remaining_messages > 0 {
                        std::thread::sleep(Duration::from_millis(interval));
                    }
                }
            }
            Err(err) => {
                eprintln!("Error receiving message from channel: {:?}", err);
                break;
            }
        }
    }
}

fn calculate_temperature(mv: f32) -> f32 {
    ((mv - V_1) / V_T) + T_1
}
