use anyhow::Result;
use clap::Parser;
use shared::client_error::ClientError;
use shared::{deserialize_message, serialize_message, MessageType};
use std::fs::{create_dir_all, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio::task;
use tokio::time::{timeout, Duration};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,

    #[arg(short, long, default_value = "11111")]
    port: u16,
}

async fn handle_message(mut reader: tokio::net::tcp::OwnedReadHalf) -> Result<(), ClientError> {
    loop {
        let mut len_bytes = [0u8; 4];
        if reader.read_exact(&mut len_bytes).await.is_err() {
            println!("Connection closed by server");
            break;
        }
        let len = u32::from_be_bytes(len_bytes) as usize;

        let mut buffer = vec![0u8; len];
        if reader.read_exact(&mut buffer).await.is_err() {
            println!("Connection closed by server");
            break;
        }

        let message = match deserialize_message(&buffer) {
            Ok(msg) => msg,
            Err(e) => {
                println!("Error deserializing message: {:?}", e);
                continue;
            }
        };

        match message {
            MessageType::Text(text) => println!("Received: {}", text),
            MessageType::Image(data) => {
                println!("Receiving image...");

                let now = chrono::Utc::now();
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
    }
    Ok(())
}

async fn send_message(
    writer: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
    message: &MessageType,
) -> Result<(), ClientError> {
    let stream_lock = timeout(Duration::from_secs(5), writer.lock()).await;
    match stream_lock {
        Ok(mut writer) => {
            let serialized = serialize_message(message).map_err(ClientError::from)?;

            let len = serialized.len() as u32;
            writer.write(&len.to_be_bytes()).await?;
            writer.write_all(&serialized).await?;
        }
        Err(_) => {
            println!("Timeout while waiting for lock.");
            return Err(ClientError::Other(
                "Timeout while waiting for lock".to_string(),
            ));
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ClientError> {
    let args = Args::parse();
    let address = format!("{}:{}", args.ip, args.port);

    println!("Arguments: {:?}", args);

    let stream = TcpStream::connect(&address).await?;
    let (mut reader, writer) = stream.into_split();

    let writer = Arc::new(Mutex::new(writer));

    // Poslání autentizačního tokenu
    {
        let mut writer = writer.lock().await;
        let token = b"SECRET_TOKEN";
        let len = token.len() as u32;
        writer.write(&len.to_be_bytes()).await?;
        writer.write_all(token).await?;

        let mut response_len_bytes = [0u8; 4];
        reader.read_exact(&mut response_len_bytes).await?;
        let response_len = u32::from_be_bytes(response_len_bytes) as usize;
        let mut response = vec![0u8; response_len];
        reader.read_exact(&mut response).await?;
        let response_str = String::from_utf8_lossy(&response);
        println!("Server response: {}", response_str);

        if !response_str.contains("Authentication Successful") {
            println!("Authentication failed.");
            return Ok(());
        }
    }

    let (sender, mut receiver) = mpsc::channel::<MessageType>(32);

    let writer_clone = Arc::clone(&writer);
    task::spawn(async move {
        while let Some(message) = receiver.recv().await {
            if let Err(e) = send_message(writer_clone.clone(), &message).await {
                println!("Error sending message: {:?}", e);
            }
        }
    });

    task::spawn(async move {
        if let Err(e) = handle_message(reader).await {
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
            MessageType::Text(message_str)
        };

        sender
            .send(message)
            .await
            .map_err(|e| ClientError::Other(e.to_string()))?;
    }

    Ok(())
}
