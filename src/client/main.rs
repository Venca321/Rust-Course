//! Tohle je program vytvořený v Rustu na základě kurzu

use bincode;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::net::TcpStream;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "localhost")]
    ip: String,

    #[arg(short, long, default_value = "11111")]
    port: u16,
}

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

fn send_message(address: &str, message: &MessageType) {
    let serialized = serialize_message(message);
    let mut stream = TcpStream::connect(address).unwrap();

    // Send the length of the serialized message (as 4-byte value).
    let len = serialized.len() as u32;
    stream.write(&len.to_be_bytes()).unwrap();

    // Send the serialized message.
    stream.write_all(&serialized).unwrap(); // Opraveno: použití &serialized místo serialized.as_bytes()
}

fn main() {
    let args = Args::parse();
    println!("Arguments: {:?}", args);

    let message = MessageType::Text("Hello, world!".to_string());
    let address = format!("{}:{}", args.ip, args.port);
    send_message(&address, &message);
}
