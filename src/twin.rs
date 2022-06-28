use crate::Message;
use azure_iot_sdk::client::*;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub fn update(
    state: TwinUpdateState,
    desired: serde_json::Value,
    tx_app2client: Arc<Mutex<Sender<Message>>>,
) -> Result<(), IotError> {
    desired_general_consent(state, desired, tx_app2client)
}

fn desired_general_consent(
    _state: TwinUpdateState,
    _desired: serde_json::Value,
    _tx_app2client: Arc<Mutex<Sender<Message>>>,
) -> Result<(), IotError> {
/*     if let Some(consents) = match state {
        TwinUpdateState::Partial => desired["general_consent"].as_array(),
        TwinUpdateState::Complete => desired["desired"]["general_consent"].as_array(),
    } {
    } else {
        info!("no general consent defined in desired properties");
    }

    report_general_consent(Arc::clone(&tx_app2client))?; */

    Ok(())
}
