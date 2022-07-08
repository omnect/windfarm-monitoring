use crate::Message;
use azure_iot_sdk::client::*;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub fn get_direct_methods(_tx_app2client: Arc<Mutex<Sender<Message>>>) -> Option<DirectMethodMap> {
    None
}
