use crate::Message;
use azure_iot_sdk::client::*;
use serde_json::json;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub fn update(
    _state: TwinUpdateState,
    _desired: serde_json::Value,
    _tx_app2client: Arc<Mutex<Sender<Message>>>,
) -> Result<(), IotError> {
    Ok(())
}

pub fn report_versions(tx_app2client: Arc<Mutex<Sender<Message>>>) -> Result<(), IotError> {
    tx_app2client
        .lock()
        .unwrap()
        .send(Message::Reported(json!({
            "module-version": env!("CARGO_PKG_VERSION"),
            "azure-sdk-version": IotHubClient::get_sdk_version_string()
        })))?;

    Ok(())
}
