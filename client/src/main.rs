use anyhow::Result;
use chrono::prelude::*;
use clap::Parser;
use shared::client_error::ClientError;
use shared::{deserialize_message, serialize_message, MessageType};
use std::fs::{create_dir_all, File};
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::thread;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,

    #[arg(short, long, default_value = "11111")]
    port: u16,
}

fn handle_message(mut stream: TcpStream) -> Result<(), ClientError> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    let mut buffer = vec![0u8; len];
    stream.read_exact(&mut buffer)?;

    let message = deserialize_message(&buffer).map_err(ClientError::from)?;
    match message {
        MessageType::Text(text) => println!("{}", text),
        MessageType::Image(data) => {
            println!("Receiving image...");

            let now = Utc::now();
            let timestamp_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

            create_dir_all("images")?;
            let mut destination_file =
                File::create(Path::new(&format!("images/{}.png", timestamp_str)))?;
            destination_file.write_all(&data)?;
        }
        MessageType::File(filename, data) => {
            println!("Receiving {}", filename);

            create_dir_all("files")?;
            let mut destination_file = File::create(Path::new(&format!("files/{}", filename)))?;
            destination_file.write_all(&data)?;
        }
    }

    Ok(())
}

fn send_message(mut stream: TcpStream, message: &MessageType) -> Result<(), ClientError> {
    let serialized = serialize_message(message).map_err(ClientError::from)?;

    let len = serialized.len() as u32;
    stream.write(&len.to_be_bytes())?;
    stream.write_all(&serialized)?;

    Ok(())
}

fn main() -> Result<(), ClientError> {
    let args = Args::parse();
    let address = format!("{}:{}", args.ip, args.port);

    println!("Arguments: {:?}", args);

    let stream = TcpStream::connect(&address)?;

    let recv_stream = stream.try_clone()?;
    thread::spawn(move || loop {
        if let Err(e) = handle_message(recv_stream.try_clone().unwrap()) {
            println!("Error handling message: {:?}", e);
        }
    });

    loop {
        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;

        let message_str = user_input.trim().to_string();
        let message = if message_str.starts_with(".file") {
            let filename = message_str.trim_start_matches(".file").trim().to_string();
            let mut source_file = File::open(Path::new(&filename))?;
            let mut buffer = Vec::new();
            source_file.read_to_end(&mut buffer)?;
            MessageType::File(filename, buffer)
        } else if message_str.starts_with(".image") {
            let filename = message_str.trim_start_matches(".image").trim().to_string();
            if !filename.ends_with(".png") {
                println!("Only PNG images are supported.");
                continue;
            }
            let mut source_file = File::open(Path::new(&filename))?;
            let mut buffer = Vec::new();
            source_file.read_to_end(&mut buffer)?;
            MessageType::Image(buffer)
        } else if message_str.starts_with(".quit") {
            println!("Quitting...");
            break;
        } else {
            MessageType::Text(user_input)
        };

        send_message(stream.try_clone()?, &message)?;
    }

    Ok(())
}
