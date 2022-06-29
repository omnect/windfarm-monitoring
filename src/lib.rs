pub mod client;
pub mod direct_methods;
pub mod message;
pub mod prometheus;
#[cfg(feature = "systemd")]
pub mod systemd;
pub mod twin;
use azure_iot_sdk::client::*;
use client::{Client, Message};
use log::error;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

const RX_CLIENT2APP_TIMEOUT: u64 = 1;

#[tokio::main]
pub async fn run() -> Result<(), IotError> {
    let mut client = Client::new();
    let (tx_client2app, rx_client2app) = mpsc::channel();
    let (tx_app2client, rx_app2client) = mpsc::channel();
    let tx_app2client = Arc::new(Mutex::new(tx_app2client));
    let methods = direct_methods::get_direct_methods(Arc::clone(&tx_app2client));

    let result;

    client.run(None, methods, tx_client2app, rx_app2client);
    prometheus::run();

    loop {
        match rx_client2app.try_recv() {
            Ok(Message::Authenticated) => {
                #[cfg(feature = "systemd")]
                systemd::notify_ready();

                if let Err(e) = twin::report_version(Arc::clone(&tx_app2client)) {
                    error!("Couldn't report version: {}", e);
                }
            }
            Ok(Message::Unauthenticated(reason)) => {
                result = Err(IotError::from(format!(
                    "No connection. Reason: {:?}",
                    reason
                )));

                break;
            }
            Ok(Message::Desired(state, desired)) => {
                if let Err(e) = twin::update(state, desired, Arc::clone(&tx_app2client)) {
                    error!("Couldn't handle twin desired: {}", e);
                }
            }
            Ok(Message::C2D(msg)) => {
                message::update(msg, Arc::clone(&tx_app2client));
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                error!("iot channel unexpectedly closed by client");
                result = Err(Box::new(mpsc::TryRecvError::Disconnected));

                break;
            }
            _ => {}
        }

        thread::sleep(Duration::from_secs(RX_CLIENT2APP_TIMEOUT));
    }

    client.stop().await?;

    result
}
