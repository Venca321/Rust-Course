use clap::Parser;
use shared::{deserialize_message, serialize_message, MessageType};
use std::collections::HashMap;
use std::io;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
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

/// Funkce pro zpracování klientského spojení
fn handle_client(mut stream: TcpStream) -> Result<MessageType, io::Error> {
    // Načtení délky zprávy (4 bajty)
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;

    // Načtení samotné zprávy podle délky
    let mut buffer = vec![0u8; len];
    stream.read_exact(&mut buffer)?;

    // Deserializace zprávy
    Ok(deserialize_message(&buffer))
}

/// Funkce pro poslech a přijímání spojení na dané adrese
fn listen_and_accept(address: &str) {
    // Vytvoření TcpListeneru pro danou adresu
    let listener = match TcpListener::bind(address) {
        Ok(lis) => lis,
        Err(_) => {
            println!("Could not start a server!");
            return;
        }
    };
    // Sdílená mapa pro uchovávání připojených klientů
    let clients = Arc::new(Mutex::new(HashMap::new()));

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        let addr = match stream.peer_addr() {
            Ok(a) => a,
            Err(_) => continue,
        };
        let clients = Arc::clone(&clients);

        {
            let mut clients_guard = clients.lock().unwrap();
            clients_guard.insert(addr.clone(), stream.try_clone().unwrap());
            println!("Connected clients: {}", clients_guard.len());
        }

        // Spuštění nového vlákna pro každého klienta
        thread::spawn(move || loop {
            let message = match handle_client(stream.try_clone().unwrap()) {
                Ok(message) => message,
                Err(err) => {
                    println!("Could not read message: {:?}", err);
                    break;
                }
            };
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

/// Funkce pro odesílání zprávy klientovi
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
    let address = format!("{}:{}", args.ip, args.port);

    println!("Listening on: {}", address);
    listen_and_accept(&address);
}
