use chrono::prelude::*;
use clap::Parser;
use shared::{deserialize_message, serialize_message, MessageType};
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::thread;

/// Struktura pro uchovávání argumentů příkazové řádky
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// IP adresa serveru
    #[arg(short, long, default_value = "127.0.0.1")]
    ip: String,

    /// Port serveru
    #[arg(short, long, default_value = "11111")]
    port: u16,
}

/// Funkce pro zpracování příchozí zprávy od serveru
fn handle_message(mut stream: TcpStream) {
    // Načtení délky zprávy (4 bajty)
    let mut len_bytes = [0u8; 4];
    if stream.read_exact(&mut len_bytes).is_err() {
        println!("Could not read message length.");
        return;
    }
    let len = u32::from_be_bytes(len_bytes) as usize;

    // Načtení samotné zprávy podle délky
    let mut buffer = vec![0u8; len];
    if stream.read_exact(&mut buffer).is_err() {
        println!("Could not read message.");
        return;
    }

    // Deserializace zprávy
    let message = deserialize_message(&buffer);
    match message {
        MessageType::Text(text) => println!("{}", text),
        MessageType::Image(data) => {
            println!("Receiving image...");

            let now = Utc::now();
            let timestamp_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

            // Vytvoření adresáře pro obrázky
            create_dir_all("images").unwrap();

            // Vytvoření souboru pro obrázek
            let mut destination_file =
                match File::create(Path::new(&format!("images/{}.png", timestamp_str))) {
                    Ok(file) => file,
                    Err(_) => {
                        println!("Could not create image file.");
                        return;
                    }
                };

            // Zapsání obrázku do souboru
            if destination_file.write_all(&data).is_err() {
                println!("Could not write image.");
            }
        }
        MessageType::File(filename, data) => {
            println!("Receiving {}", filename);

            // Vytvoření adresáře pro soubory
            if create_dir_all("files").is_err() {
                println!("Could not create directory.");
                return;
            }

            // Vytvoření souboru a zapsání dat do souboru
            let mut destination_file = match File::create(Path::new(&format!("files/{}", filename)))
            {
                Ok(file) => file,
                Err(_) => {
                    println!("Could not create file.");
                    return;
                }
            };
            if destination_file.write_all(&data).is_err() {
                println!("Could not write file.");
            }
        }
    }
}

/// Funkce pro odesílání zprávy serveru
fn send_message(mut stream: TcpStream, message: &MessageType) {
    let serialized = serialize_message(message);

    // Odeslání délky serializované zprávy (4 bajty)
    let len = serialized.len() as u32;
    if stream.write(&len.to_be_bytes()).is_err() {
        println!("Could not send message length.");
        return;
    }

    // Odeslání samotné serializované zprávy
    if stream.write_all(&serialized).is_err() {
        println!("Could not send message.");
    }
}

fn main() {
    // Parsování argumentů příkazové řádky
    let args = Args::parse();
    println!("Arguments: {:?}", args);

    let address = format!("{}:{}", args.ip, args.port);

    // Připojení k serveru
    let stream = match TcpStream::connect(&address) {
        Ok(stream) => stream,
        Err(_) => {
            println!("Could not connect to server.");
            return;
        }
    };

    // Klonování streamu pro přijímání zpráv
    let recv_stream = match stream.try_clone() {
        Ok(stream) => stream,
        Err(_) => {
            println!("Could not clone stream.");
            return;
        }
    };

    // Spuštění vlákna pro zpracování příchozích zpráv
    thread::spawn(move || loop {
        handle_message(recv_stream.try_clone().unwrap());
    });

    // Hlavní smyčka pro odesílání zpráv
    loop {
        let message: MessageType;
        let mut user_input = String::new();
        std::io::stdin().read_line(&mut user_input).unwrap();
        let message_str = user_input.trim().to_string();

        if message_str.starts_with(".file") {
            // Odesílání souboru
            let filename = message_str.trim_start_matches(".file").trim().to_string();
            let mut source_file = File::open(Path::new(&filename)).unwrap();
            let mut buffer = Vec::new();
            match source_file.read_to_end(&mut buffer) {
                Ok(_) => {}
                Err(_) => {
                    println!("Could not read file.");
                    continue;
                }
            };
            message = MessageType::File(filename, buffer);
        } else if message_str.starts_with(".image") {
            // Odesílání obrázku
            let filename = message_str.trim_start_matches(".image").trim().to_string();

            if !filename.ends_with(".png") {
                println!("Only PNG images are supported.");
                continue;
            }

            let mut source_file = match File::open(Path::new(&filename)) {
                Ok(file) => file,
                Err(_) => {
                    println!("File not found.");
                    continue;
                }
            };
            let mut buffer = Vec::new();
            source_file.read_to_end(&mut buffer).unwrap();
            message = MessageType::Image(buffer);
        } else if message_str.starts_with(".quit") {
            // Ukončení programu
            println!("Quitting...");
            break;
        } else {
            // Odesílání textové zprávy
            message = MessageType::Text(user_input);
        }

        let send_stream = stream.try_clone().unwrap();
        send_message(send_stream, &message);
    }
}
