use chrono::{Timelike, Utc};
use futures_executor::block_on;
use lazy_static::lazy_static;
use log::{error, info};
use rand::distributions::Uniform;
use rand::{thread_rng, Rng};
use time::format_description::well_known::Rfc3339;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tokio::time::Duration;

use azure_iot_sdk::client::IotMessage;
use serde::Serialize;

lazy_static! {
    static ref DEVICE_ID: String = std::env::var("IOTEDGE_DEVICEID").unwrap();
    static ref IOTHUB: String = std::env::var("IOTEDGE_IOTHUBHOSTNAME").unwrap();
    static ref MODULE_ID: String = std::env::var("IOTEDGE_MODULEID").unwrap();
}

#[derive(Serialize)]
struct Label {
    edge_device: String,
    iothub: String,
    module_name: String,
}

#[derive(Serialize)]
struct Metric {
    #[serde(rename = "TimeGeneratedUtc")]
    time_generated_utc: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Value")]
    value: f64,
    #[serde(rename = "Labels")]
    labels: Label,
}

impl Metric {
    fn new(time: String, name: &str, value: f64) -> Metric {
        Metric {
            time_generated_utc: time,
            name: name.to_string(),
            value,
            labels: Label {
                edge_device: DEVICE_ID.to_string(),
                iothub: IOTHUB.to_string(),
                module_name: MODULE_ID.to_string(),
            },
        }
    }
}

#[derive(Default)]
pub struct MetricsProvider {
    sim: Option<JoinHandle<()>>,
}

impl MetricsProvider {
    pub fn new() -> Self {
        MetricsProvider::default()
    }

    pub fn run(&mut self, tx_outgoing_message: Sender<IotMessage>, location: serde_json::Value) {
        self.sim = Some(tokio::spawn(MetricsProvider::data_collector(
            tx_outgoing_message.clone(),
            location["latitude"].as_f64().unwrap(),
            location["longitude"].as_f64().unwrap(),
        )));
    }

    pub async fn stop(&mut self) {
        if let Some(sim) = self.sim.as_mut() {
            sim.abort();
            sim.await.unwrap_or_else(|e| {
                if !e.is_cancelled() {
                    error!("thread terminated with error: {}", e.to_string());
                }
            });
        }
    }

    async fn data_collector(
        tx_outgoing_message: Sender<IotMessage>,
        latitude: f64,
        longitude: f64,
    ) {
        // configure interval of wind speed and wind direction samples in seconds
        let mut collect_interval = tokio::time::interval(Duration::from_secs(60));

        // init simulation ranges
        let wind_speed_range: Uniform<f64> = Uniform::new_inclusive(0.0, 10.0);
        let wind_direction_range: Uniform<i64> = Uniform::new_inclusive(0, 359);
        let wind_speed_per_hour: Vec<f64> = thread_rng()
            .sample_iter(wind_speed_range)
            .take(24)
            .collect();
        let wind_direction_per_hour: Vec<i64> = thread_rng()
            .sample_iter(wind_direction_range)
            .take(24)
            .collect();

        info!("LATITUDE: {latitude} LONGITUDE: {longitude}");

        loop {
            collect_interval.tick().await;

            let mut wind_speed: f64 = 0.0;
            let mut wind_direction: i64 = 0;
            // get wind speed of current hour
            // apply random deviation of -5.0 to 5.0 percent
            match wind_speed_per_hour.get(Utc::now().hour() as usize) {
                Some(v) => wind_speed = v + (v * thread_rng().gen_range(-5.0..5.0) / 100.0),
                _ => error!("couldn't generate wind speed"),
            }

            // get wind direction of current hour
            // apply random deviation of -5.0 to 5.0 percent
            match wind_direction_per_hour.get(Utc::now().hour() as usize) {
                Some(v) => {
                    wind_direction =
                        v + (*v as f64 * thread_rng().gen_range(-5.0..5.0) / 100.0) as i64
                }
                _ => error!("couldn't generate wind direction"),
            }

            let time = match time::OffsetDateTime::now_utc().format(&Rfc3339) {
                Ok(time) => time,
                Err(e) => {
                    error!("timestamp could not be generated: {e}");
                    String::from("")
                }
            };

            let metric_list = vec![
                Metric::new(time.clone(), "latitude", latitude),
                Metric::new(time.clone(), "longitude", longitude),
                Metric::new(time.clone(), "wind_speed", wind_speed),
                Metric::new(time.clone(), "wind_direction", wind_direction as f64),
            ];

            match serde_json::to_vec(&metric_list) {
                Ok(json) => {
                    match IotMessage::builder()
                        .set_body(json)
                        .set_content_type("application/json")
                        .set_content_encoding("utf-8")
                        .set_output_queue("metrics")
                        .build()
                    {
                        Ok(msg) => {
                            let _ = tx_outgoing_message.send(msg).await;
                        }
                        Err(e) => error!("telemetry message could not be transmitted: {e}"),
                    }
                }
                Err(e) => error!("metrics list could not be converted to vector: {e}"),
            }
        }
    }
}

impl Drop for MetricsProvider {
    fn drop(&mut self) {
        block_on(async { self.stop().await });
    }
}
