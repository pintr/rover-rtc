use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};

use bincode::config::{self, Configuration};

const BINCODE_CONFIG: Configuration = config::standard();

#[derive(Debug, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct Payload {
    pub data: Vec<u8>,
    pub timestamp: i64,
}

impl Payload {
    pub fn new(data: &[u8]) -> Payload {
        Self {
            data: data.to_vec(),
            timestamp: Utc::now().timestamp_nanos_opt().unwrap_or(0),
        }
    }

    pub fn data(&self) -> String {
        String::from_utf8_lossy(&self.data).to_string()
    }

    pub fn timestamp(&self) -> String {
        Utc.timestamp_nanos(self.timestamp).to_rfc3339()
    }

    pub fn latency(&self) -> String {
        (Utc::now() - Utc.timestamp_nanos(self.timestamp)).to_string()
    }

    pub fn serialize(payload: Payload) -> Vec<u8> {
        bincode::encode_to_vec(payload, BINCODE_CONFIG).expect("Serialization failed")
    }
    /// Deserialize from received bytes
    pub fn deserialize(bytes: Vec<u8>) -> Self {
        let (payload, _): (Payload, usize) =
            bincode::decode_from_slice(&bytes, BINCODE_CONFIG).expect("Deserialization failed");
        payload
    }
}
