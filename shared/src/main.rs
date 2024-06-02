use bincode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageType {
    Text(String),
    Image(Vec<u8>),
    File(String, Vec<u8>),
}

pub fn serialize_message(message: &MessageType) -> Vec<u8> {
    bincode::serialize(&message).unwrap()
}

pub fn deserialize_message(data: &[u8]) -> MessageType {
    bincode::deserialize(&data).unwrap()
}

fn main() {}
