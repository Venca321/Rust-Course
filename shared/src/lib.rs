use bincode;

pub mod client_error;
pub mod server_error;

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MessageType {
    Text(String),
    Image(Vec<u8>),
    File(String, Vec<u8>),
}

pub fn serialize_message(message: &MessageType) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(&message)
}

pub fn deserialize_message(data: &[u8]) -> Result<MessageType, bincode::Error> {
    bincode::deserialize(&data)
}
