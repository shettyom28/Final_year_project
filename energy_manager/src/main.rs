use reqwest;
use roxmltree;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use chrono::{Utc, Duration as ChronoDuration, Timelike, Local};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// --- CONFIGURATION ---
const MQTT_BROKER: &str = "localhost";
const MQTT_PORT: u16 = 1883;
const PRICE_THRESHOLD: f64 = 14.80;

const ENTSOE_TOKEN: &str = "9f99eba8-076d-4ec2-a1e7-f620fd8d1510";
const AREA_CODE: &str = "10Y1001A1001A59C";

async fn get_irish_electricity_price() -> Result<f64, Box<dyn std::error::Error>> {
    let now = Utc::now();
    let current_hour = now.hour();

    let start = now.format("%Y%m%d0000").to_string();
    let end = (now + ChronoDuration::days(1)).format("%Y%m%d0000").to_string();

    let url = format!(
        "https://web-api.tp.entsoe.eu/api?securityToken={}&documentType=A44&in_Domain={}&out_Domain={}&periodStart={}&periodEnd={}",
        ENTSOE_TOKEN, AREA_CODE, AREA_CODE, start, end
    );

    let response = reqwest::get(url).await?.error_for_status()?.text().await?;

    let doc = roxmltree::Document::parse(&response)?;

    let target_position = (current_hour + 1).to_string();

    let price_node = doc.descendants().find(|n| {
        n.tag_name().name() == "Point" &&
        n.children().any(|c| c.tag_name().name() == "position" && c.text() == Some(&target_position))
    });

    if let Some(node) = price_node {
        let price_str = node.children()
            .find(|c| c.tag_name().name() == "price.amount")
            .and_then(|c| c.text())
            .ok_or("Found Point but no price.amount")?;

        let price_mwh: f64 = price_str.parse()?;
        Ok(price_mwh / 10.0)
    } else {
        Err(format!("Position {} not found in XML. Is the Day-Ahead data published yet?", target_position).into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut mqttoptions = MqttOptions::new("pi_brain", MQTT_BROKER, MQTT_PORT);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    tokio::spawn(async move {
        loop {
            if let Err(e) = eventloop.poll().await {
                eprintln!("MQTT Connection Error: {:?}", e);
            }
        }
    });

    println!("Irish Energy Manager Active");
    println!("Targeting Threshold: {} c/kWh", PRICE_THRESHOLD);

    loop {
        match get_irish_electricity_price().await {
            Ok(current_price) => {
                // --- LED Decision ---
                let command = if current_price <= PRICE_THRESHOLD {
                    "1" // Cheap -> LED ON
                } else {
                    "0" // Expensive -> LED OFF
                };

                // --- Record timestamp for latency test ---
                let start_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();

                // --- Send command:timestamp as payload ---
                // e.g. "1:1748293847123" or "0:1748293847123"
                // ESP32 splits on ':' to get both the command and sent time
                let payload = format!("{}:{}", command, start_time);

                client
                    .publish("esp32/led_cmd", QoS::AtMostOnce, false, payload.as_str())
                    .await?;

                println!(
                    "[{}] Price: {:.2} c/kWh | Command: {} | Sent at: {} ms | Payload: {}",
                    Local::now().format("%H:%M:%S"),
                    current_price,
                    command,
                    start_time,
                    payload
                );
            }
            Err(e) => eprintln!("Failed to fetch price: {:?}", e),
        }
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}