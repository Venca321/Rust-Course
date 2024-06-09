use anyhow::Result;
use clap::Parser;
use shared::server_error::ServerError;
use shared::{deserialize_message, serialize_message, MessageType};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,

    #[arg(short, long, default_value = "11111")]
    port: u16,
}

fn handle_client(mut stream: TcpStream) -> Result<MessageType, ServerError> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    let mut buffer = vec![0u8; len];
    stream.read_exact(&mut buffer)?;

    deserialize_message(&buffer).map_err(ServerError::from)
}

fn listen_and_accept(address: &str) -> Result<(), ServerError> {
    let listener = TcpListener::bind(address)?;

    let clients = Arc::new(Mutex::new(HashMap::new()));

    for stream in listener.incoming() {
        let stream = stream?;
        let addr = stream.peer_addr()?;
        let clients = Arc::clone(&clients);

        {
            let mut clients_guard = clients.lock().unwrap();
            clients_guard.insert(addr.clone(), stream.try_clone()?);
            println!("Connected clients: {}", clients_guard.len());
        }

        thread::spawn(move || loop {
            match handle_client(stream.try_clone().unwrap()) {
                Ok(message) => {
                    println!("Received message: {:?}", message);
                    let clients_guard = clients.lock().unwrap();
                    for (client_addr, client_stream) in clients_guard.iter() {
                        if client_addr != &addr {
                            println!("Sending message to: {}", client_addr);
                            if let Err(e) =
                                send_message(client_stream.try_clone().unwrap(), &message)
                            {
                                println!("Error sending message: {:?}", e);
                            }
                        }
                    }
                }
                Err(err) => {
                    println!("Error handling client: {:?}", err);
                    break;
                }
            }
        });
    }

    Ok(())
}

fn send_message(mut stream: TcpStream, message: &MessageType) -> Result<(), ServerError> {
    let serialized = serialize_message(message).map_err(ServerError::from)?;

    let len = serialized.len() as u32;
    stream.write(&len.to_be_bytes())?;
    stream.write_all(&serialized)?;

    Ok(())
}

fn main() -> Result<(), ServerError> {
    let args = Args::parse();
    let address = format!("{}:{}", args.ip, args.port);

    println!("Listening on: {}", address);
    listen_and_accept(&address)?;

    Ok(())
}
