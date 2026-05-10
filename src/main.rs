// Cargo.toml
//
// [package]
// name = "simple_proxy"
// version = "0.1.0"
// edition = "2024"
//
// [dependencies]
// tokio = { version = "1", features = ["full"] }

//use std::fs;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

struct Config {
    listen: String,
}

use std::{env, fs, path::PathBuf};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //let config = load_config("proxy.conf")?;
    let config = load_config()?;

    let listener = TcpListener::bind(&config.listen).await?;

    println!("proxy listening on {}", config.listen);

    loop {
        let (client, addr) = listener.accept().await?;

        println!("accepted: {}", addr);

        tokio::spawn(async move {
            if let Err(e) = handle_client(client).await {
                eprintln!("error: {}", e);
            }
        });
    }
}

fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let home = env::var("HOME")?;

    let path: PathBuf = [
        home.as_str(),
        ".config",
        "ggg",
        "proxy.conf",
    ]
    .iter()
    .collect();

    let text = fs::read_to_string(&path)?;

    let mut listen = "0.0.0.0:8080".to_string();

    for line in text.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(value) = line.strip_prefix("listen=") {
            listen = value.trim().to_string();
        }
    }

    Ok(Config { listen })
}


async fn handle_client(
    mut client: TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 8192];

    let n = client.read(&mut buf).await?;

    if n == 0 {
        return Ok(());
    }

    let req = String::from_utf8_lossy(&buf[..n]);

    let first_line = req.lines().next().unwrap_or("");

    println!("request: {}", first_line);

    if first_line.starts_with("CONNECT ") {
        handle_connect(client, &req).await?;
    } else {
        handle_http(client, &buf[..n], &req).await?;
    }

    Ok(())
}

async fn handle_connect(
    mut client: TcpStream,
    req: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let first_line = req.lines().next().unwrap();

    let target = first_line
        .split_whitespace()
        .nth(1)
        .ok_or("invalid CONNECT")?;

    println!("CONNECT target: {}", target);

    let mut server = TcpStream::connect(target).await?;

    client
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await?;

    tokio::io::copy_bidirectional(&mut client, &mut server).await?;

    Ok(())
}

async fn handle_http(
    mut client: TcpStream,
    first_packet: &[u8],
    req: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let host = extract_host(req).ok_or("Host header not found")?;

    let target = if host.contains(':') {
        host
    } else {
        format!("{}:80", host)
    };

    println!("HTTP target: {}", target);

    let mut server = TcpStream::connect(target).await?;

    server.write_all(first_packet).await?;

    tokio::io::copy_bidirectional(&mut client, &mut server).await?;

    Ok(())
}

fn extract_host(req: &str) -> Option<String> {
    for line in req.lines() {
        if line.to_ascii_lowercase().starts_with("host:") {
            return Some(line[5..].trim().to_string());
        }
    }

    None
}
