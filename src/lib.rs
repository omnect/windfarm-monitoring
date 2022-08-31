pub mod client;
pub mod direct_methods;
pub mod message;
pub mod metrics_provider;
#[cfg(feature = "systemd")]
pub mod systemd;
pub mod twin;
use azure_iot_sdk::client::*;
use client::{Client, Message};
use log::{debug, error};
use metrics_provider::MetricsProvider;
use rand::{thread_rng, Rng};
use serde_json::json;
use std::sync::Once;
use std::sync::{mpsc, Arc, Mutex};

static AUTHENTICATED_ONCE: Once = Once::new();
static TWIN_READY_ONCE: Once = Once::new();

#[tokio::main]
pub async fn run() -> Result<(), IotError> {
    let mut client = Client::new();
    let mut metrics_provider = MetricsProvider::new();
    let (tx_client2app, rx_client2app) = mpsc::channel();
    let (tx_app2client, rx_app2client) = mpsc::channel();
    let tx_app2client = Arc::new(Mutex::new(tx_app2client));
    let methods = direct_methods::get_direct_methods(Arc::clone(&tx_app2client));

    client.run(None, methods, tx_client2app, rx_app2client);

    for msg in rx_client2app {
        match msg {
            Message::Authenticated => {
                AUTHENTICATED_ONCE.call_once(|| {
                    #[cfg(feature = "systemd")]
                    systemd::notify_ready();

                    if let Err(e) = twin::report_versions(Arc::clone(&tx_app2client)) {
                        error!("Couldn't report version: {}", e);
                    }
                });
            }
            Message::Unauthenticated(reason) => {
                if !matches!(reason, UnauthenticatedReason::ExpiredSasToken) {
                    return Err(IotError::from(format!(
                        "No connection. Reason: {:?}",
                        reason
                    )));
                }
            }
            Message::Desired(state, twin) => {
                if let TwinUpdateState::Complete = state {
                    TWIN_READY_ONCE.call_once(|| {
                        let mut location = twin["reported"]["location"].clone();
                        if serde_json::Value::Null == location {
                            location = json!({ "location": {"latitude": thread_rng().gen_range(53.908754f64..53.956915f64), "longitude": thread_rng().gen_range(8.594901f64..8.741848f64)} });

                            tx_app2client
                                .lock()
                                .unwrap()
                                .send(Message::Reported(location.clone()))
                                .unwrap();

                            location = location["location"].clone();
                        }

                        metrics_provider.run(location);
                    });
                }

                if let Err(e) = twin::update(state, twin, Arc::clone(&tx_app2client)) {
                    error!("Couldn't handle twin desired: {}", e);
                }
            }
            Message::C2D(msg) => {
                message::update(msg, Arc::clone(&tx_app2client));
            }
            _ => debug!("Application received unhandled message"),
        }
    }

    metrics_provider.stop().await;
    client.stop().await
}
