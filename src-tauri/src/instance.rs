//! Keeps one running copy, through a Unix socket rather than a D-Bus name. Strict snap confinement
//! refuses to let an application own a bus name it did not declare, and a failure there would leave
//! two copies running two schedulers. A socket in the runtime directory works the same everywhere.

use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

pub enum Instance {
    /// This process is the running copy.
    Primary(UnixListener),
    /// Another copy is running and has received the message.
    Forwarded,
    /// The socket could not be set up. The application runs anyway rather than refusing to start.
    Unavailable,
}

fn socket_path() -> PathBuf {
    let directory = std::env::var_os("XDG_RUNTIME_DIR").map_or_else(std::env::temp_dir, PathBuf::from);
    directory.join("dayweave.sock")
}

pub fn acquire(message: &str) -> Instance {
    let path = socket_path();
    match UnixListener::bind(&path) {
        Ok(listener) => Instance::Primary(listener),
        Err(error) if error.kind() == ErrorKind::AddrInUse => match UnixStream::connect(&path) {
            Ok(mut stream) => match stream.write_all(message.as_bytes()) {
                Ok(()) => Instance::Forwarded,
                Err(_) => Instance::Unavailable,
            },
            // Nobody is listening, so the file was left behind by a crash.
            Err(_) => {
                let _ = std::fs::remove_file(&path);
                UnixListener::bind(&path).map_or(Instance::Unavailable, Instance::Primary)
            }
        },
        Err(_) => Instance::Unavailable,
    }
}

pub fn listen(listener: UnixListener, handle: impl Fn(&str) + Send + 'static) {
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut message = String::new();
            let mut limited = stream.take(64);
            if limited.read_to_string(&mut message).is_ok() {
                handle(message.trim());
            }
        }
    });
}
