use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;

fn cachecat_ping(addr: &str) -> std::io::Result<bool> {
    let mut stream = TcpStream::connect(addr)?;

    stream.write_all(b"*1\r\n$4\r\nPING\r\n")?;

    let mut buffer = [0u8; 512];
    let n = stream.read(&mut buffer)?;

    let response = String::from_utf8_lossy(&buffer[..n]);

    Ok(response.starts_with("+PONG"))
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <host> <port>", args[0]);
        std::process::exit(1);
    }

    let host = &args[1];
    let port = &args[2];

    let addr = format!("{}:{}", host, port);

    match cachecat_ping(&addr) {
        Ok(true) => {
            println!("OK");
        }
        Ok(false) => {
            eprintln!("Unexpected response from CacheCat");
            std::process::exit(1);
        }
        Err(err) => {
            eprintln!("Failed to ping CacheCat: {}", err);
            std::process::exit(1);
        }
    }

    Ok(())
}
