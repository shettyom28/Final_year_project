use esp_idf_hal::gpio::*;
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi, Configuration, ClientConfiguration, AuthMethod};
use esp_idf_svc::mqtt::client::*;
use esp_idf_svc::sntp::{EspSntp, SyncStatus};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use esp_idf_sys as _;

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("=== ESP32-S3: Energy Price LED Receiver ===");

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // --- 1. LED Setup ---
    let mut led = PinDriver::output(peripherals.pins.gpio2)?;

    // --- 2. WiFi Setup ---
    let mut esp_wifi = EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?;
    let mut wifi = BlockingWifi::wrap(esp_wifi, sysloop.clone())?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: "iot".try_into().unwrap(),
        password: "admonishes11kneecapping".try_into().unwrap(),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    }))?;

    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    log::info!("✓ WiFi Connected! IP: {}", ip_info.ip);

    // --- SNTP Clock Sync ---
    let sntp = EspSntp::new_default()?;
    log::info!("Waiting for SNTP sync...");
    while sntp.get_sync_status() != SyncStatus::Completed {
        thread::sleep(Duration::from_millis(100));
    }
    log::info!("✓ SNTP Synchronized");

    // --- 3. MQTT Setup ---
    let broker_url = "mqtt://136.206.12.185:1883";

    let (mut mqtt_client, mut connection) = EspMqttClient::new(
        broker_url,
        &MqttClientConfiguration::default(),
    )?;

    // Background thread: receive messages, measure latency, control LED
    thread::spawn(move || {
        log::info!("MQTT Listening Thread Started");
        let mut measurement_count = 0;

        while let Ok(event) = connection.next() {
            if let EventPayload::Received { data, .. } = event.payload() {
                // Record arrival time immediately
                let arrival_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();

                if let Ok(payload_str) = std::str::from_utf8(data) {
                    // Payload format is "command:timestamp"
                    // e.g. "1:1748293847123" or "0:1748293847123"
                    let parts: Vec<&str> = payload_str.splitn(2, ':').collect();

                    if parts.len() == 2 {
                        let command = parts[0];
                        let sent_time_str = parts[1];

                        // --- Latency Measurement ---
                        if let Ok(sent_time) = sent_time_str.parse::<u128>() {
                            let latency = arrival_time - sent_time;
                            measurement_count += 1;

                            log::info!(
                                "[Measurement {}] Latency: {} ms (sent: {} ms, arrived: {} ms)",
                                measurement_count,
                                latency,
                                sent_time,
                                arrival_time
                            );

                            if latency > 300 {
                                log::warn!(
                                    "High latency detected: {} ms - possible WiFi jitter",
                                    latency
                                );
                            }
                        }

                        // --- LED Control ---
                        match command {
                            "1" => {
                                led.set_high().unwrap();
                                log::info!("[Command] Energy is CHEAP: LED ON");
                            }
                            "0" => {
                                led.set_low().unwrap();
                                log::info!("[Command] Energy is EXPENSIVE: LED OFF");
                            }
                            _ => {
                                log::warn!("Unknown command received: {}", command);
                            }
                        }
                    } else {
                        log::warn!("Unexpected payload format: {}", payload_str);
                    }
                }
            }
        }
    });

    // Wait for MQTT connection to stabilise before subscribing
    thread::sleep(Duration::from_secs(5));
    log::info!("Subscribing to topic: esp32/led_cmd");
    mqtt_client.subscribe("esp32/led_cmd", QoS::AtMostOnce)?;

    // --- 4. Main Loop ---
loop {
    if !wifi.is_connected().unwrap_or(false) {
        log::warn!("WiFi connection lost! Reconnecting...");
        
        if wifi.connect().is_ok() {
            wifi.wait_netif_up().unwrap_or(());
            let ip = wifi.wifi().sta_netif().get_ip_info().unwrap().ip;
            log::info!("✓ Reconnected! IP: {}", ip);
            
            // Re-subscribe after reconnection
            thread::sleep(Duration::from_secs(5));
            if let Err(e) = mqtt_client.subscribe("esp32/led_cmd", QoS::AtMostOnce) {
                log::error!("Re-subscribe failed: {:?}", e);
            } else {
                log::info!("✓ Re-subscribed to esp32/led_cmd");
            }
        }
    }
    thread::sleep(Duration::from_secs(5));
}
}