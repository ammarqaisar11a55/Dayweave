//! Serves the RPC dispatcher over HTTP on localhost, so the interface can run in an ordinary browser
//! against the real core and a real database. For development and testing only: it is behind the
//! `dev-bridge` feature and is never packaged.
//!
//! usage: dayweave-bridge [data-dir] [port]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use dayweave_core::Core;
use serde_json::{json, Value};

fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn handle(core: &Core, mut stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut length = 0;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let header = line.trim();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    let request: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let method = request["method"].as_str().unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or(json!({}));
    match core.rpc(method, params) {
        Ok(result) => respond(&mut stream, "200 OK", &result.to_string()),
        Err(error) => respond(&mut stream, "400 Bad Request", &json!(error).to_string()),
    }
    Ok(())
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let data_dir =
        arguments.next().map_or_else(|| std::env::temp_dir().join("dayweave-bridge"), PathBuf::from);
    let port = arguments.next().unwrap_or_else(|| "1421".into());

    let core = Arc::new(Core::open(&data_dir).expect("open the database"));
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind the port");
    println!("Serving {} on http://127.0.0.1:{port}", data_dir.display());

    let scheduler = core.clone();
    std::thread::spawn(move || loop {
        match scheduler.tick() {
            Ok(notices) => notices.iter().for_each(|notice| println!("notice: {}", notice.title)),
            Err(error) => eprintln!("tick failed: {error}"),
        }
        std::thread::sleep(Duration::from_secs(5));
    });

    for stream in listener.incoming().flatten() {
        let core = core.clone();
        std::thread::spawn(move || {
            let _ = handle(&core, stream);
        });
    }
}
