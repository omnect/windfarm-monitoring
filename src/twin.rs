use crate::metrics_provider::MetricsProvider;
use crate::Message;
use anyhow::{Context, Result};
use azure_iot_sdk::client::*;
use log::{info, warn};
use once_cell::sync::OnceCell;
use rand::{thread_rng, Rng};
use serde_json::json;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, MutexGuard, Once};

static INSTANCE: OnceCell<Mutex<Twin>> = OnceCell::new();
static LOCATION_ONCE: Once = Once::new();

pub struct TwinInstance {
    inner: &'static Mutex<Twin>,
}

pub fn get_or_init(tx: Option<&Sender<Message>>) -> TwinInstance {
    if let Some(tx) = tx {
        TwinInstance {
            inner: INSTANCE.get_or_init(|| {
                Mutex::new(Twin {
                    tx: Some(tx.clone()),
                    metrics_provider: Some(MetricsProvider::new()),
                })
            }),
        }
    } else {
        TwinInstance {
            inner: INSTANCE.get().unwrap(),
        }
    }
}

struct TwinLock<'a> {
    inner: MutexGuard<'a, Twin>,
}

impl TwinInstance {
    fn lock(&self) -> TwinLock<'_> {
        TwinLock {
            inner: self.inner.lock().unwrap_or_else(|e| e.into_inner()),
        }
    }

    pub fn report(&self, property: &ReportProperty) -> Result<()> {
        self.lock().inner.report(property)
    }

    pub fn update(&self, state: TwinUpdateState, desired: serde_json::Value) -> Result<()> {
        self.lock().inner.update(state, desired)
    }

    pub fn cloud_message(&self, msg: IotMessage) {
        warn!(
            "received unexpected C2D message with \n body: {:?}\n properties: {:?} \n system properties: {:?}",
            std::str::from_utf8(&msg.body).unwrap(),
            msg.properties,
            msg.system_properties
        );
    }
}

#[derive(Default)]
struct Twin {
    tx: Option<Sender<Message>>,
    metrics_provider: Option<MetricsProvider>,
}

pub enum ReportProperty {
    Versions,
}

impl Twin {
    fn update(&mut self, state: TwinUpdateState, desired: serde_json::Value) -> Result<()> {
        match state {
            TwinUpdateState::Partial => Ok(()),
            TwinUpdateState::Complete => {
                self.update_location(desired["reported"]["location"].as_object())
            }
        }
    }

    fn report(&mut self, property: &ReportProperty) -> Result<()> {
        match property {
            ReportProperty::Versions => self.report_versions().context("Couldn't report version"),
        }
    }

    fn report_versions(&mut self) -> Result<()> {
        self.report_impl(json!({
            "module-version": env!("CARGO_PKG_VERSION"),
            "azure-sdk-version": IotHubClient::get_sdk_version_string()
        }))
        .context("report_versions")
    }

    fn update_location(
        &mut self,
        coordinates: Option<&serde_json::Map<String, serde_json::Value>>,
    ) -> Result<()> {
        LOCATION_ONCE.call_once(|| {
            let location = match coordinates {
                Some(values) => json!({ "location": values }),
                _ => json!({
                    "location": {
                        "latitude": thread_rng().gen_range(53.908754f64..53.956915f64),
                        "longitude": thread_rng().gen_range(8.594901f64..8.741848f64)
                    }
                }),
            };

            self.report_impl(location.clone())
                .context("update_location")
                .unwrap();

            self.metrics_provider
                .as_mut()
                .unwrap()
                .run(location["location"].clone());
        });
        Ok(())
    }

    fn report_impl(&mut self, value: serde_json::Value) -> Result<()> {
        info!("report: {}", value);

        self.tx
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("tx channel missing"))?
            .send(Message::Reported(value))
            .map_err(|err| err.into())
    }
}
