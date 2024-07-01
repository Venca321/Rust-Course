use anyhow::Result;
use clap::Parser;
use shared::server_error::ServerError;
use shared::{deserialize_message, serialize_message, MessageType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
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

async fn handle_client(
    reader: Arc<Mutex<tokio::net::tcp::OwnedReadHalf>>,
    sender: mpsc::Sender<(MessageType, std::net::SocketAddr)>,
    addr: std::net::SocketAddr,
) -> Result<(), ServerError> {
    let mut reader = reader.lock().await;
    loop {
        let mut len_bytes = [0u8; 4];
        if reader.read_exact(&mut len_bytes).await.is_err() {
            break; // Connection closed
        }
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut buffer = vec![0u8; len];
        if reader.read_exact(&mut buffer).await.is_err() {
            break; // Connection closed
        }

        let message = deserialize_message(&buffer).map_err(ServerError::from)?;
        sender
            .send((message, addr))
            .await
            .map_err(|e| ServerError::Other(e.to_string()))?;
    }
    Ok(())
}

async fn listen_and_accept(address: &str) -> Result<(), ServerError> {
    let listener = TcpListener::bind(address).await?;
    let clients: Arc<
        Mutex<
            HashMap<
                std::net::SocketAddr,
                (
                    Arc<Mutex<tokio::net::tcp::OwnedReadHalf>>,
                    Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
                ),
            >,
        >,
    > = Arc::new(Mutex::new(HashMap::new()));
    let (message_sender, mut message_receiver) =
        mpsc::channel::<(MessageType, std::net::SocketAddr)>(32);

    let clients_clone = Arc::clone(&clients);
    task::spawn(async move {
        while let Some((message, sender_addr)) = message_receiver.recv().await {
            println!("Received message: {:?}", message);
            let clients = clients_clone.lock().await;
            for (client_addr, (_client_reader, client_writer)) in clients.iter() {
                if client_addr != &sender_addr {
                    println!("Sending message to: {}", client_addr);
                    let mut writer = client_writer.lock().await;
                    if let Err(e) = send_message(&mut writer, &message).await {
                        println!("Error sending message: {:?}", e);
                    }
                    println!("Message sent");
                }
            }
        }
    });

    loop {
        let (stream, addr) = listener.accept().await?;
        let (reader, writer) = stream.into_split();
        let clients = Arc::clone(&clients);
        let message_sender = message_sender.clone();
        {
            let mut clients_guard = clients.lock().await;
            clients_guard.insert(
                addr,
                (Arc::new(Mutex::new(reader)), Arc::new(Mutex::new(writer))),
            );
            println!("Connected clients: {}", clients_guard.len());
        }

        let client_reader = Arc::clone(&clients.lock().await.get(&addr).unwrap().0);
        task::spawn(async move {
            if let Err(err) = handle_client(client_reader, message_sender, addr).await {
                println!("Error handling client {}: {:?}", addr, err);
                clients.lock().await.remove(&addr);
            }
        });
    }
}

async fn send_message(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    message: &MessageType,
) -> Result<(), ServerError> {
    let serialized = serialize_message(message).map_err(ServerError::from)?;

    let len = serialized.len() as u32;
    writer.write(&len.to_be_bytes()).await?;
    writer.write_all(&serialized).await?;

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
