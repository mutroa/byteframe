//! Simple echo server and client example using byteframe protocol.
//!
//! Run the server:
//!   cargo run --example simple_echo server
//!
//! In another terminal, run the client:
//!   cargo run --example simple_echo client

use byteframe::{Packet, PacketReader, PacketWriter};
use std::env;
use std::io::{self, BufRead};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} [server|client]", args[0]);
        std::process::exit(1);
    }

    match args[1].as_str() {
        "server" => run_server(),
        "client" => run_client(),
        _ => {
            eprintln!("Unknown mode: {}", args[1]);
            eprintln!("Usage: {} [server|client]", args[0]);
            std::process::exit(1);
        }
    }
}

fn run_server() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("Echo server listening on 127.0.0.1:8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New client connected: {}", stream.peer_addr().unwrap());
                thread::spawn(|| handle_client(stream));
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }
}

fn handle_client(stream: TcpStream) {
    let peer = stream.peer_addr().unwrap();
    let read_stream = stream.try_clone().unwrap();
    let write_stream = stream;

    let mut reader = PacketReader::new(read_stream);
    let mut writer = PacketWriter::new(write_stream);

    // Send a welcome message
    if let Err(e) = writer.write_packet(&Packet::Message("Welcome to echo server!".into())) {
        eprintln!("[{}] Failed to send welcome: {}", peer, e);
        return;
    }
    writer.flush().unwrap();

    loop {
        match reader.read_packet() {
            Ok(packet) => {
                println!("[{}] Received: {:?}", peer, packet);

                // Echo the packet back
                match packet {
                    Packet::Ping => {
                        writer.write_packet(&Packet::Pong).unwrap();
                    }
                    other => {
                        writer.write_packet(&other).unwrap();
                    }
                }
                writer.flush().unwrap();
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    println!("[{}] Client disconnected", peer);
                } else {
                    eprintln!("[{}] Read error: {}", peer, e);
                }
                break;
            }
        }
    }
}

fn run_client() {
    let stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    println!("Connected to server");

    let read_stream = stream.try_clone().unwrap();
    let write_stream = stream;

    let mut reader = PacketReader::new(read_stream);
    let mut writer = PacketWriter::new(write_stream);

    // Spawn a thread to receive packets
    let receiver = thread::spawn(move || {
        loop {
            match reader.read_packet() {
                Ok(packet) => {
                    println!("← Received: {:?}", packet);
                }
                Err(e) => {
                    if e.kind() == io::ErrorKind::UnexpectedEof {
                        println!("Server disconnected");
                    } else {
                        eprintln!("Error: {}", e);
                    }
                    break;
                }
            }
        }
    });

    // Main thread sends packets based on user input
    println!("\nCommands:");
    println!("  ping          - Send a Ping packet");
    println!("  data <bytes>  - Send Data packet (e.g., 'data 01 02 03')");
    println!("  <message>     - Send Message packet");
    println!("  quit          - Exit\n");

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "quit" {
            break;
        }

        let packet = if trimmed == "ping" {
            Packet::Ping
        } else if let Some(hex_str) = trimmed.strip_prefix("data ") {
            let bytes: Vec<u8> = hex_str
                .split_whitespace()
                .filter_map(|s| u8::from_str_radix(s, 16).ok())
                .collect();
            Packet::Data(bytes)
        } else {
            Packet::Message(trimmed.to_string())
        };

        println!("→ Sending: {:?}", packet);
        if let Err(e) = writer.write_packet(&packet) {
            eprintln!("Send error: {}", e);
            break;
        }
        writer.flush().unwrap();
    }

    println!("Exiting...");
    receiver.join().ok();
}
