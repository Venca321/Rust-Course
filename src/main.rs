//! Tohle je program vytvořený v Rustu na základě kurzu

use bincode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
enum MessageType {
    Text(String),
    Image(Vec<u8>),
    File(String, Vec<u8>),
}

fn serialize_message(message: &MessageType) -> Vec<u8> {
    bincode::serialize(&message).unwrap()
}

fn deserialize_message(data: &[u8]) -> MessageType {
    bincode::deserialize(&data).unwrap()
}

fn main() {
    let message = MessageType::Text("Hello, world!".to_string());
    println!("Message: {:?}", message);

    let serialized = serialize_message(&message);
    println!("Serialized: {:?}", serialized);

    let deserialized: MessageType = deserialize_message(&serialized);
    println!("Deserialized: {:?}", deserialized);
}
