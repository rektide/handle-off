#![cfg(unix)]
use std::{
    io::{BufRead, BufReader, Read},
    process::{Command, Stdio},
};

#[test]
fn independent_processes_transfer_file_custody() {
    let root =
        std::env::temp_dir().join(format!("handle-off-rights-process-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("socket");
    let file = root.join("file");
    std::fs::write(&file, b"cross-process\0\xff\n").unwrap();
    let mut receiver = Command::new(env!("CARGO_BIN_EXE_handle-off-raw-rights-example"))
        .arg("receive")
        .arg(&path)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = BufReader::new(receiver.stdout.take().unwrap());
    let mut ready = String::new();
    output.read_line(&mut ready).unwrap();
    assert_eq!(ready, "ready\n");
    let sent = Command::new(env!("CARGO_BIN_EXE_handle-off-raw-rights-example"))
        .arg("send")
        .arg(&path)
        .arg(&file)
        .status()
        .unwrap();
    assert!(sent.success());
    let mut bytes = Vec::new();
    output.read_to_end(&mut bytes).unwrap();
    assert!(receiver.wait().unwrap().success());
    assert_eq!(bytes, b"cross-process\0\xff\n");
    assert!(!path.exists());
    std::fs::remove_file(file).unwrap();
    std::fs::remove_dir(root).unwrap();
}
