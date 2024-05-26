use bincode;
use chrono::prelude::*;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
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
fn handle_message(mut stream: TcpStream) {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).unwrap();
    let len = u32::from_be_bytes(len_bytes) as usize;

    let mut buffer = vec![0u8; len];
    stream.read_exact(&mut buffer).unwrap();

    let message = deserialize_message(&buffer);
    match message {
        MessageType::Text(text) => println!("{}", text),
        MessageType::Image(data) => {
            println!("Receiving image...");

            let now = Utc::now();
            let timestamp_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

            create_dir_all("src/client/images").unwrap();

            let mut destination_file = File::create(Path::new(&format!(
                "src/client/images/{}.png",
                timestamp_str
            )))
            .unwrap();
            destination_file.write_all(&data).unwrap();
        }
        MessageType::File(filename, data) => {
            println!("Receiving {}", filename);

            create_dir_all("src/client/files").unwrap();

            let mut destination_file =
                File::create(Path::new(&format!("src/client/files/{}", filename))).unwrap();
            destination_file.write_all(&data).unwrap();
        }
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
    println!("Arguments: {:?}", args);

    let address = format!("{}:{}", args.ip, args.port);
    let stream = TcpStream::connect(address).unwrap();

    let recv_stream = stream.try_clone().unwrap();
    thread::spawn(move || loop {
        handle_message(recv_stream.try_clone().unwrap());
    });

    loop {
        let message: MessageType;
        let mut user_input = String::new();
        std::io::stdin().read_line(&mut user_input).unwrap();
        let message_str = user_input.trim().to_string();

        if message_str.starts_with(".file") {
            let filename = message_str.trim_start_matches(".file").trim().to_string();
            let mut source_file = File::open(Path::new(&filename)).unwrap();
            let mut buffer = Vec::new();
            source_file.read_to_end(&mut buffer).unwrap();
            message = MessageType::File(filename, buffer);
        } else if message_str.starts_with(".image") {
            let filename = message_str.trim_start_matches(".image").trim().to_string();

            if !filename.ends_with(".png") {
                println!("Only PNG images are supported.");
                continue;
            }

            let mut source_file = File::open(Path::new(&filename)).unwrap();
            let mut buffer = Vec::new();
            source_file.read_to_end(&mut buffer).unwrap();
            message = MessageType::Image(buffer);
        } else if message_str.starts_with(".quit") {
            println!("Quitting...");
            break;
        } else {
            message = MessageType::Text(user_input);
        }

        let send_stream = stream.try_clone().unwrap();
        send_message(send_stream, &message);
    }
}
