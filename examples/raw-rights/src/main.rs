//! Raw custody example. Run `receive SOCKET` then `send SOCKET FILE` in another process.
#[cfg(unix)]
fn main() -> std::io::Result<()> {
    use handle_off_unix_rights::{ReceiveLimits, receive, send};
    use std::{
        fs::File,
        io::{self, Write},
        os::{fd::AsFd, unix::net::UnixDatagram},
    };
    let args: Vec<_> = std::env::args_os().collect();
    match args.get(1).and_then(|arg| arg.to_str()) {
        Some("receive") if args.len() == 3 => {
            // Bind fails on existing paths; cleanup is explicit, never pre-bind removal.
            let socket = UnixDatagram::bind(&args[2])?;
            println!("ready");
            io::stdout().flush()?;
            let mut message = receive(
                &socket,
                ReceiveLimits {
                    max_payload: 64,
                    max_fds: 1,
                },
            )?;
            std::fs::remove_file(&args[2])?;
            if message.payload != b"file" || message.descriptors.len() != 1 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "expected one file",
                ));
            }
            let mut file = File::from(message.descriptors.remove(0));
            io::copy(&mut file, &mut io::stdout())?;
            Ok(())
        }
        Some("send") if args.len() == 4 => {
            let socket = UnixDatagram::unbound()?;
            socket.connect(&args[2])?;
            let file = File::open(&args[3])?;
            send(&socket, b"file", &[file.as_fd()])
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: receive SOCKET | send SOCKET FILE",
        )),
    }
}
#[cfg(not(unix))]
fn main() {
    eprintln!("Unix descriptor transfer is required");
}
