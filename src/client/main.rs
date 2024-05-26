//! Tohle je program vytvořený v Rustu na základě kurzu

use bincode;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;

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

fn handle_message(mut stream: TcpStream) {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).unwrap();
    let len = u32::from_be_bytes(len_bytes) as usize;

    let mut buffer = vec![0u8; len];
    stream.read_exact(&mut buffer).unwrap();

    let message = deserialize_message(&buffer);
    println!("Received message: {:?}", message);
}

fn send_message(mut stream: TcpStream, message: &MessageType) {
    let serialized = serialize_message(message);

    // Send the length of the serialized message (as 4-byte value).
    let len = serialized.len() as u32;
    stream.write(&len.to_be_bytes()).unwrap();

    // Send the serialized message.
    stream.write_all(&serialized).unwrap();
}

fn main() {
    let args = Args::parse();
    println!("Arguments: {:?}", args);

    let address = format!("{}:{}", args.ip, args.port);
    let stream = TcpStream::connect(address).unwrap();
    let recv_stream = stream.try_clone().unwrap();

    thread::spawn(move || {
        handle_message(recv_stream); // Use the cloned stream
    });

    loop {
        let mut user_input = String::new();
        std::io::stdin().read_line(&mut user_input).unwrap();
        let message = MessageType::Text(user_input.trim().to_string());
        println!("Sending message: {:?}", message);
        send_message(stream.try_clone().unwrap(), &message);
    }
}
