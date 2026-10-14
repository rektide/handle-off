//! A successor restoring a pending write on an actual transferred Unix stream.
//! Supply explicit connected data/control peers; production uses fresh session tokens.
#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use handle_off_handover_direct::{Correlation, Decision, Receiver};
    use handle_off_state::Payload;
    use handle_off_transport_unix::{Delivery, Endpoint, Limits};
    use std::{
        io::{self, Read, Write},
        os::{
            fd::AsFd,
            unix::net::{UnixDatagram, UnixStream},
        },
        time::Duration,
    };
    struct Pending {
        socket: UnixStream,
        bytes: Vec<u8>,
        offset: usize,
    }
    fn restore(delivery: &Delivery) -> io::Result<Pending> {
        let invalid = || {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported pending-write image",
            )
        };
        if !delivery.manifest.extensions.is_empty() || delivery.manifest.bundles.len() != 1 {
            return Err(invalid());
        }
        let bundle = &delivery.manifest.bundles[0];
        if !bundle.extensions.is_empty() || bundle.members.len() != 1 {
            return Err(invalid());
        }
        let member = &bundle.members[0];
        if member.extensions.len() != 1 {
            return Err(invalid());
        }
        let extension = &member.extensions[0];
        if extension.key != "pending"
            || extension.format.name != "example.pending-write"
            || extension.format.version != 1
        {
            return Err(invalid());
        }
        let Payload::Inline(image) = &extension.payload else {
            return Err(invalid());
        };
        if image.len() < 8 || image.len() > 4096 {
            return Err(invalid());
        }
        let offset = u64::from_le_bytes(image[..8].try_into().map_err(|_| invalid())?);
        let offset = usize::try_from(offset).map_err(|_| invalid())?;
        let bytes = image[8..].to_vec();
        if offset > bytes.len() {
            return Err(invalid());
        }
        let fd = delivery
            .handles
            .get(&member.resource)
            .ok_or_else(invalid)?
            .as_fd()
            .try_clone_to_owned()?;
        let socket = UnixStream::from(fd);
        socket.peer_addr()?; // Query only: no I/O/flag changes.
        Ok(Pending {
            socket,
            bytes,
            offset,
        })
    }
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 5 {
        return Err("usage: SUCCESSOR DATA_LOCAL DATA_PEER CONTROL_LOCAL CONTROL_PEER".into());
    }
    let data = UnixDatagram::bind(&args[1])?;
    data.connect(&args[2])?;
    data.set_read_timeout(Some(Duration::from_secs(3)))?;
    let control = UnixDatagram::bind(&args[3])?;
    control.connect(&args[4])?;
    println!("listening");
    io::stdout().flush()?;
    let mut endpoint = Endpoint::new(data, Limits::default())?;
    let offer = endpoint.receive()?;
    let correlation = Correlation::new(
        [0x42; 16],
        offer.delivery().manifest.generation.clone(),
        offer.offer_id(),
    )?;
    let mut receiver = Receiver::new(control, correlation, Duration::from_secs(3))?;
    let staged = receiver
        .stage(offer)
        .map_err(|failure| io::Error::other(format!("{}; delivery retained", failure.error)))?;
    let mut pending = None;
    receiver
        .ready(&staged, |delivery| {
            pending = Some(restore(delivery)?);
            Ok::<(), io::Error>(())
        })
        .map_err(|error| io::Error::other(format!("{error:?}; application stays dormant")))?;
    match receiver.wait_decision()? {
        Decision::Release(notice) => {
            let activated = receiver.activate(staged, notice).map_err(|failure| {
                io::Error::other(format!("{}; permission/delivery retained", failure.error))
            })?;
            let mut app = pending.take().ok_or("missing parked application")?;
            if activated.permission.correlation().generation
                != activated.delivery.manifest.generation
            {
                return Err("mismatched execution permission".into());
            }
            app.socket.write_all(&app.bytes[app.offset..])?;
            // Timeout configuration is AFTER permission, not a dormant shared-state mutation.
            app.socket.set_read_timeout(Some(Duration::from_secs(3)))?;
            let mut request = [0; 4];
            app.socket.read_exact(&mut request)?;
            if &request != b"ping" {
                return Err("unexpected live-stream request".into());
            }
            app.socket.write_all(b"pong")?;
        }
        Decision::Abort(notice) => {
            receiver
                .retire(staged, notice, || {
                    drop(pending.take());
                    Ok::<(), io::Error>(())
                })
                .map_err(|failure| {
                    io::Error::other(format!(
                        "{:?}; retained state requires explicit recovery",
                        failure.error
                    ))
                })?;
        }
    }
    // Explicit fixture cleanup; library never removes paths or application state.
    std::fs::remove_file(&args[1])?;
    std::fs::remove_file(&args[3])?;
    Ok(())
}
#[cfg(not(unix))]
fn main() {
    eprintln!("this connected Unix-stream handover example requires Unix");
}
