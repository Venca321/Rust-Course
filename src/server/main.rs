//! Tohle je program vytvořený v Rustu na základě kurzu

use bincode;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};

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

/*
fn serialize_message(message: &MessageType) -> Vec<u8> {
    bincode::serialize(&message).unwrap()
}
*/

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

    let mut clients: HashMap<SocketAddr, TcpStream> = HashMap::new();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let addr = stream.peer_addr().unwrap();
        clients.insert(addr.clone(), stream);

        println!("Connection from: {}", addr);

        let message = handle_client(clients.get(&addr).unwrap().try_clone().unwrap());
        println!("{:?}", message);
    }
}

fn main() {
    let args = Args::parse();
    println!("Arguments: {:?}", args);

    let address = format!("{}:{}", args.ip, args.port);
    listen_and_accept(&address);
}
