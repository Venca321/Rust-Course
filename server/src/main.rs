use anyhow::Result;
use clap::Parser;
use shared::server_error::ServerError;
use shared::{deserialize_message, serialize_message, MessageType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio::task;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,

    #[arg(short, long, default_value = "11111")]
    port: u16,
}

async fn handle_client(stream: Arc<Mutex<TcpStream>>) -> Result<MessageType, ServerError> {
    let mut stream = stream.lock().await;
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    let mut buffer = vec![0u8; len];
    stream.read_exact(&mut buffer).await?;

    deserialize_message(&buffer).map_err(ServerError::from)
}

async fn listen_and_accept(address: &str) -> Result<(), ServerError> {
    let listener = TcpListener::bind(address).await?;

    let clients = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (stream, addr) = listener.accept().await?;
        let clients = Arc::clone(&clients);
        let stream = Arc::new(Mutex::new(stream));

        {
            let mut clients_guard = clients.lock().await;
            clients_guard.insert(addr, Arc::clone(&stream));
            println!("Connected clients: {}", clients_guard.len());
        }

        let (sender, mut receiver) = mpsc::channel::<MessageType>(32);
        let recv_stream = Arc::clone(&stream);

        task::spawn(async move {
            while let Some(message) = receiver.recv().await {
                let clients_guard = clients.lock().await;
                for (client_addr, client_stream) in clients_guard.iter() {
                    if client_addr != &addr {
                        println!("Sending message to: {}", client_addr);
                        if let Err(e) = send_message(client_stream.clone(), &message).await {
                            println!("Error sending message: {:?}", e);
                        }
                    }
                }
            }
        });

        task::spawn(async move {
            loop {
                match handle_client(Arc::clone(&recv_stream)).await {
                    Ok(message) => {
                        println!("Received message: {:?}", message);
                        if let Err(e) = sender.send(message).await {
                            println!("Error sending to channel: {:?}", e);
                        }
                    }
                    Err(err) => {
                        println!("Error handling client: {:?}", err);
                        break;
                    }
                }
            }
        });
    }
}

async fn send_message(
    stream: Arc<Mutex<TcpStream>>,
    message: &MessageType,
) -> Result<(), ServerError> {
    let mut stream = stream.lock().await;
    let serialized = serialize_message(message).map_err(ServerError::from)?;

    let len = serialized.len() as u32;
    stream.write(&len.to_be_bytes()).await?;
    stream.write_all(&serialized).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let args = Args::parse();
    let address = format!("{}:{}", args.ip, args.port);

    println!("Listening on: {}", address);
    listen_and_accept(&address).await?;

    Ok(())
}
