# 🦀 Rust-Based Event-Driven IoT System for Real-Time Energy Monitoring and Control

> **Final Year Project** — BEng Electronic & Computer Engineering (IoT)  
> **Author:** Om Vasanth Shetty  
> **Supervisor:** Dr. Derek Molloy  
> **Institution:** Dublin City University  
> **Year:** 2026

---

## 📖 Overview

This project presents a decentralised, event-driven IoT system for real-time energy monitoring and price-aware load actuation, built **entirely in Rust**. A Raspberry Pi 5 gateway queries the [ENTSO-E Transparency Platform](https://transparency.entsoe.eu/) for Irish day-ahead electricity prices, runs a decision engine, and publishes commands via MQTT to an ESP32-S3 edge node which actuates a physical load based on configurable price thresholds.

All control logic runs **locally within the home network** — no cloud dependency — ensuring low latency, offline resilience, and data privacy by design.

---

## 🏗️ System Architecture

```
ENTSO-E API (Day-ahead prices)
        │  HTTPS / REST
        ▼
┌─────────────────────────────────────┐
│        Raspberry Pi 5 — Gateway     │
│  ┌──────────────────┐  ┌──────────────────────┐  │
│  │ Rust Decision    │─▶│ Mosquitto MQTT Broker │  │
│  │ Engine (Tokio)   │  │ (localhost:1883)       │  │
│  └──────────────────┘  └──────────────────────┘  │
└─────────────────────────────────────┘
        │  MQTT QoS 1 (Local WiFi)
        ▼
┌─────────────────────────┐
│  ESP32-S3 — Edge Node   │
│  Subscribes to commands │
│  Controls GPIO2 LED     │
└─────────────────────────┘
        │  GPIO
        ▼
   Load / LED Actuator
```

---

## ✨ Features

- ⚡ **Real-time price-aware actuation** — fetches Irish SEM day-ahead prices from ENTSO-E API hourly
- 🦀 **End-to-end Rust** — firmware (ESP32-S3) and gateway (Raspberry Pi) both written in Rust
- 🔒 **Memory safe** — Rust's ownership model eliminates buffer overflows and data races at compile time
- 📡 **MQTT communication** — lightweight publish-subscribe with QoS 1 guaranteed delivery for commands
- 🏠 **Local-first** — all logic runs on the home network; only ENTSO-E query needs internet
- 📊 **Validated performance** — mean one-way MQTT latency of **51.33 ms** on local WiFi

---

## 🧰 Tech Stack

| Component | Technology |
|---|---|
| Edge firmware | Rust (`esp-idf-hal`, `esp-idf-svc`) |
| Gateway software | Rust (`tokio`, `rumqttc`, `reqwest`, `roxmltree`) |
| Edge hardware | ESP32-S3 (dual-core Xtensa, 512KB SRAM) |
| Gateway hardware | Raspberry Pi 5 (4-core ARM, 8GB RAM) |
| MQTT broker | Mosquitto (localhost) |
| Communication | MQTT over local WiFi |
| Pricing data | ENTSO-E Transparency Platform REST API |
| Async runtime | Tokio |

---

## 📁 Repository Structure

```
├── esp32_project/          # ESP32-S3 edge node firmware
│   ├── src/
│   │   └── main.rs         # WiFi init, MQTT client, LED control
│   ├── Cargo.toml
│   └── sdkconfig.defaults
│
├── energy_manager/         # Raspberry Pi gateway software
│   ├── src/
│   │   └── main.rs         # ENTSO-E API fetch, XML parse, MQTT publish
│   └── Cargo.toml
│
└── README.md
```

---

## 🚀 Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) installed
- [espup](https://github.com/esp-rs/espup) — Espressif Rust toolchain installer
- [espflash](https://github.com/esp-rs/espflash) — flashing tool
- Raspberry Pi 5 running Raspberry Pi OS
- Mosquitto MQTT broker installed on Raspberry Pi

---

### ESP32-S3 Firmware Setup

```bash
# Install the Espressif Rust toolchain
cargo install espup
espup install
source ~/export-esp.sh

# Clone the repo
git clone https://github.com/shettyom28/Final_year_project.git
cd Final_year_project/esp32_project

# Edit WiFi credentials and broker IP in src/main.rs
# Then build and flash
espflash flash --monitor target/xtensa-esp32s3-espidf/debug/esp32_project
```

---

### Raspberry Pi Gateway Setup

```bash
# Install Mosquitto broker
sudo apt install mosquitto mosquitto-clients

# Configure Mosquitto to accept local connections
echo "listener 1883\nallow_anonymous true" | sudo tee /etc/mosquitto/conf.d/local.conf
sudo systemctl restart mosquitto

# Disable WiFi power saving for reliability
sudo iwconfig wlan0 power off

# Set your ENTSO-E API token in energy_manager/src/main.rs
# Then run the gateway
cd Final_year_project/energy_manager
cargo run
```

---

## 📊 Results

### MQTT Latency Test

| Measurement | Sent (ms) | Arrived (ms) | Latency (ms) |
|---|---|---|---|
| 1 | 1774721781069 | 1774721781088 | **19** |
| 2 | 1774721797740 | 1774721797779 | **39** |
| 3 | 1774721813479 | 1774721813548 | **69** |
| 4 | 1774721831616 | 1774721831673 | **56** |
| 5 | 1774721858608 | 1774721858707 | **99** |
| 6 | 1774721874349 | 1774721874375 | **26** |
| **Mean** | | | **51.33 ms** |

### End-to-End Integration Test

| Scenario | Threshold | Price | Expected | Received | LED |
|---|---|---|---|---|---|
| A — Low price | 15 c/kWh | 14.80 c/kWh | 1 | 1 | ✅ ON |
| B — High price | 10 c/kWh | 14.80 c/kWh | 0 | 0 | ✅ OFF |

---

## ⚠️ Known Limitations

- MQTT broker runs without authentication — suitable for development only. Add TLS + username/password for production.
- ENTSO-E API token is hardcoded — move to an environment variable before sharing code publicly.
- System currently controls a single LED as a proof of concept.

---

## 🔮 Future Work

- Mobile companion app for real-time price monitoring and device control
- TLS encryption and MQTT authentication for production security
- Control of higher-power loads (water heaters, EV chargers)
- Integration of real-time carbon intensity data for environmental optimisation
- Persistent data logging for historical cost analysis

---

## 📄 License

This project was developed as an academic final year project at Dublin City University. All rights reserved.

---

## 📬 Contact

**Om Vasanth Shetty**  
📧 om.shetty222@gmail.com  
🔗 [LinkedIn](https://www.linkedin.com/in/om-vasanth-shetty/)  
🐙 [GitHub](https://github.com/shettyom28)
