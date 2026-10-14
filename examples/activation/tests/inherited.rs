#![cfg(unix)]
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    process::{Command, Stdio},
    time::Duration,
};

fn manager(mode: &str) -> Command {
    let mut command = Command::new(env!(
        "CARGO_BIN_EXE_handle-off-synthetic-activation-manager"
    ));
    command
        .arg(env!("CARGO_BIN_EXE_handle-off-activation-example"))
        .arg(mode);
    command
}
#[test]
fn named_listener_at_second_inherited_slot_accepts_after_exec() {
    let mut child = manager("valid")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut ready = String::new();
    output.read_line(&mut ready).unwrap();
    let mut fields = ready.split_whitespace();
    assert_eq!(fields.next(), Some("ready"));
    let address = fields.next().expect("listener address");
    assert_eq!(fields.next(), Some("count=2"));
    assert_eq!(fields.next(), Some("cloexec=true"));
    let mut socket = TcpStream::connect(address).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    socket.write_all(b"ping").unwrap();
    let mut reply = [0; 4];
    socket.read_exact(&mut reply).unwrap();
    assert_eq!(&reply, b"pong");
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn duplicate_tags_fail_selection_and_absent_or_foreign_pid_does_not_claim() {
    let output = manager("duplicate").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Ambiguous"));
    for mode in ["absent", "foreign"] {
        let output = manager(mode).output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("no activation"));
    }
}
