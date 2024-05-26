//! Tohle je program vytvořený v Rustu na základě kurzu

use bincode;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")] //localhost is not working
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

fn handle_client(mut stream: TcpStream) -> MessageType {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).unwrap();
    let len = u32::from_be_bytes(len_bytes) as usize;

    let mut buffer = vec![0u8; len];
    stream.read_exact(&mut buffer).unwrap();

    deserialize_message(&buffer)
}

fn listen_and_accept(address: &str) {
    let listener = TcpListener::bind(address).unwrap();
    let clients = Arc::new(Mutex::new(HashMap::new()));

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let addr = stream.peer_addr().unwrap();
        let clients = Arc::clone(&clients);

        {
            let mut clients_guard = clients.lock().unwrap();
            clients_guard.insert(addr.clone(), stream.try_clone().unwrap());
            println!("Connected clients: {}", clients_guard.len());
        }

        thread::spawn(move || loop {
            let message = handle_client(stream.try_clone().unwrap());
            println!("Received message: {:?}", message);

            let clients_guard = clients.lock().unwrap();
            for (client_addr, client_stream) in clients_guard.iter() {
                if client_addr != &addr {
                    println!("Sending message to: {}", client_addr);
                    send_message(client_stream.try_clone().unwrap(), &message);
                }
            }
        });
    }
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
    let address = format!("{}:{}", args.ip, args.port);

    println!("Listening on: {}", address);
    listen_and_accept(&address);
}
