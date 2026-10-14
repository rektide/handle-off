//! A one-connection TCP activation consumer. No binding or manager custody policy.
#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use handle_off_activation::{Limits, Snapshot};
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    let metadata = Snapshot::capture()
        .parse(std::process::id(), Limits::default())?
        .ok_or("no activation for this process")?;
    // SAFETY: this example's deployment contract supplies valid, otherwise unowned
    // inherited FD 3+ directly after exec. No startup library or threads claim them.
    // Environment parsing alone is not sufficient to establish this precondition.
    let mut activation = unsafe { metadata.claim()? };
    let all_cloexec = activation.descriptors().iter().all(|entry| {
        rustix::io::fcntl_getfd(&entry.handle)
            .is_ok_and(|flags| flags.contains(rustix::io::FdFlags::CLOEXEC))
    });
    let selected = activation.take_unique("listener")?;
    if rustix::net::sockopt::socket_type(&selected.handle)? != rustix::net::SocketType::STREAM
        || !rustix::net::sockopt::socket_acceptconn(&selected.handle)?
    {
        return Err("listener tag does not identify a listening stream socket".into());
    }
    let listener = TcpListener::from(selected.handle);
    println!(
        "ready {} count={} cloexec={all_cloexec}",
        listener.local_addr()?,
        activation.descriptors().len() + 1
    );
    std::io::stdout().flush()?;
    let (mut socket, _) = listener.accept()?;
    let mut bytes = [0; 4];
    socket.read_exact(&mut bytes)?;
    if &bytes != b"ping" {
        return Err("unexpected fixture request".into());
    }
    socket.write_all(b"pong")?;
    Ok(())
}
#[cfg(not(unix))]
fn main() {
    eprintln!("activation descriptors require Unix");
}
