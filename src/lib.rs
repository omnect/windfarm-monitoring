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
use std::sync::{mpsc, Arc, Mutex};

#[tokio::main]
pub async fn run() -> Result<(), IotError> {
    let mut client = Client::new();
    let mut metrics_provider = MetricsProvider::new();
    let (tx_client2app, rx_client2app) = mpsc::channel();
    let (tx_app2client, rx_app2client) = mpsc::channel();
    let tx_app2client = Arc::new(Mutex::new(tx_app2client));
    let methods = direct_methods::get_direct_methods(Arc::clone(&tx_app2client));
    let mut result = Ok(());

    client.run(None, methods, tx_client2app, rx_app2client);
    metrics_provider.run();

    for msg in rx_client2app {
        match msg {
            Message::Authenticated => {
                #[cfg(feature = "systemd")]
                systemd::notify_ready();

                if let Err(e) = twin::report_version(Arc::clone(&tx_app2client)) {
                    error!("Couldn't report version: {}", e);
                }
            }
            Message::Unauthenticated(reason) => {
                result = Err(IotError::from(format!(
                    "No connection. Reason: {:?}",
                    reason
                )));

                break;
            }
            Message::Desired(state, desired) => {
                if let Err(e) = twin::update(state, desired, Arc::clone(&tx_app2client)) {
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
    client.stop().await?;

    result
}
