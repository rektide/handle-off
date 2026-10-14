//! Linux sample receiver with explicit socket paths, not a discovery policy.
//! Invocation: RECEIVER LOCAL_SOCKET PEER_SOCKET (peer must already be bound).
#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use handle_off_state::Payload;
    use handle_off_state_memfd::BorrowedImage;
    use handle_off_transport_unix::{Endpoint, Limits};
    use std::{
        io::Write,
        os::{fd::AsFd, unix::net::UnixDatagram},
        time::Duration,
    };
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 3 {
        return Err("usage: RECEIVER LOCAL_SOCKET PEER_SOCKET".into());
    }
    let socket = UnixDatagram::bind(&args[1])?;
    socket.connect(&args[2])?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut endpoint = Endpoint::new(socket, Limits::default())?;
    println!("ready");
    std::io::stdout().flush()?;
    let offer = endpoint.receive()?;
    let delivery = offer.delivery();
    if delivery.manifest.bundles.len() != 2 {
        return Err("sample expects two bundles".into());
    }
    let mut images = Vec::new();
    for bundle in &delivery.manifest.bundles {
        if bundle.members.len() != 1 {
            return Err("sample expects one member per bundle".into());
        }
        for extension in &bundle.members[0].extensions {
            if extension.format.name != "example.bytes" || extension.format.version != 1 {
                return Err("unsupported application state format".into());
            }
            let Payload::ResourceRange {
                resource,
                offset,
                length,
            } = &extension.payload
            else {
                return Err("sample expects resource-backed state".into());
            };
            let handle = delivery.handles.get(resource).ok_or("missing image")?;
            let image = BorrowedImage::from_borrowed(handle.as_fd(), 1024)?;
            images.push(image.read_range(*offset, *length)?);
        }
    }
    if images.len() != 2 {
        return Err("sample expects two independent images".into());
    }
    // Application validates formats, seals and image bounds BEFORE accepting custody.
    let _accepted = offer.accept()?;
    std::fs::remove_file(&args[1])?; // explicit retirement of this example's own path
    for image in images {
        std::io::stdout().write_all(&image)?;
    }
    Ok(())
}
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("this multiple-memfd example requires Linux");
}
