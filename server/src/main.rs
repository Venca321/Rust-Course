use anyhow::Result;
use clap::Parser;
use dotenv::dotenv;
use shared::server_error::ServerError;
use shared::{deserialize_message, serialize_message, MessageType};
use sqlx::sqlite::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio::task;

/// Struktura pro uchování argumentů příkazového řádku
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,

    #[arg(short, long, default_value = "11111")]
    port: u16,

    #[arg(short, long, default_value = "sqlite:chat.db")]
    database_url: String,
}

/// Funkce pro zpracování přijatých zpráv od klienta
///
/// # Arguments
///
/// * `reader` - Asynchronní čtecí část TCP spojení
/// * `writer` - Asynchronní zapisovací část TCP spojení
/// * `sender` - Kanál pro odesílání zpráv
/// * `addr` - Adresa klienta
async fn handle_client(
    reader: Arc<Mutex<tokio::net::tcp::OwnedReadHalf>>,
    writer: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
    sender: mpsc::Sender<(MessageType, std::net::SocketAddr)>,
    addr: std::net::SocketAddr,
    _pool: SqlitePool,
) -> Result<(), ServerError> {
    // Ověření klienta
    {
        let mut reader = reader.lock().await;
        let mut writer = writer.lock().await;
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut buffer = vec![0u8; len];
        reader.read_exact(&mut buffer).await?;
        let token = String::from_utf8(buffer)
            .map_err(|_| ServerError::Other("Invalid token format".to_string()))?;

        if token != "SECRET_TOKEN" {
            // Zde ověřujeme token
            writer.write_all(&4u32.to_be_bytes()).await?;
            writer.write_all(b"FAIL").await?;
            return Err(ServerError::Other("Authentication failed".to_string()));
        }

        let serialized = b"Authentication Successful";
        let len = serialized.len() as u32;
        writer.write(&len.to_be_bytes()).await?;
        writer.write_all(serialized).await?;
    }

    // Pokračování standardní komunikace
    loop {
        let mut reader = reader.lock().await;
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

/// Funkce pro poslouchání a přijímání příchozích spojení
///
/// # Arguments
///
/// * `address` - Adresa, na které server poslouchá
async fn listen_and_accept(address: &str, pool: SqlitePool) -> Result<(), ServerError> {
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
            println!("Received message from {}: {:?}", sender_addr, message);
            let clients = clients_clone.lock().await;
            for (client_addr, (_client_reader, client_writer)) in clients.iter() {
                if client_addr != &sender_addr {
                    let mut writer = client_writer.lock().await;
                    if let Err(e) = send_message(&mut writer, &message).await {
                        println!("Error sending message to {}: {:?}", client_addr, e);
                    }
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
        let client_writer = Arc::clone(&clients.lock().await.get(&addr).unwrap().1);
        let pool = pool.clone();
        task::spawn(async move {
            if let Err(err) =
                handle_client(client_reader, client_writer, message_sender, addr, pool).await
            {
                println!("Error handling client {}: {:?}", addr, err);
                clients.lock().await.remove(&addr);
            }
        });
    }
}

/// Funkce pro odesílání zpráv na klienty
///
/// # Arguments
///
/// * `writer` - Asynchronní zapisovací část TCP spojení
/// * `message` - Typ zprávy k odeslání
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

/// Inicializuje databázi a vytváří potřebné tabulky.
///
/// # Arguments
///
/// * `pool` - Databázový pool pro připojení k SQLite databázi
///
/// # Returns
///
/// Vrací `Result<(), ServerError>`, který indikuje úspěch nebo chybu během inicializace.
///
/// # Errors
///
/// Vrací `ServerError`, pokud se nepodaří vytvořit tabulky v databázi.
///
/// # Example
///
/// ```rust
/// let pool = SqlitePool::connect(&database_url).await?;
/// init_db(&pool).await?;
/// ```
async fn init_db(pool: &SqlitePool) -> Result<(), ServerError> {
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT NOT NULL UNIQUE
        );
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER,
            content TEXT NOT NULL,
            FOREIGN KEY(user_id) REFERENCES users(id)
        );
        ",
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    dotenv().ok();
    let args = Args::parse();
    let address = format!("{}:{}", args.ip, args.port);

    let pool = SqlitePool::connect(&args.database_url).await?;
    init_db(&pool).await?;

    println!("Listening on: {}", address);
    listen_and_accept(&address, pool).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_db_connection() -> Result<(), ServerError> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        init_db(&pool).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_send_message() -> Result<(), ServerError> {
        let message = MessageType::Text("Hello, World!".to_string());
        let (mut client, mut server) = duplex(64);

        // Send message to server side (server acts as the client for this test)
        let serialized = serialize_message(&message).map_err(ServerError::from)?;
        let len = serialized.len() as u32;
        server.write_all(&len.to_be_bytes()).await?;
        server.write_all(&serialized).await?;

        // Read from client side (client acts as the server for this test)
        let mut len_bytes = [0u8; 4];
        client.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut buffer = vec![0u8; len];
        client.read_exact(&mut buffer).await?;

        let received_message = deserialize_message(&buffer).map_err(ServerError::from)?;
        assert_eq!(message, received_message);

        Ok(())
    }
}
