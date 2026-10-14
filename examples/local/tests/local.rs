#![cfg(unix)]
use std::{
    ffi::OsString,
    io::Write,
    os::unix::ffi::OsStringExt,
    process::{Command, Stdio},
};

#[test]
fn real_pipes_and_non_utf8_capture() {
    // Run in a non-UTF-8 cwd without mutating the test process's cwd/environment.
    let root = std::env::temp_dir().join(format!("handle-off-local-{}", std::process::id()));
    let cwd = root.join(OsString::from_vec(b"cwd-\xff".to_vec()));
    std::fs::create_dir_all(&cwd).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_handle-off-local-example"))
        .arg(OsString::from_vec(b"arg-\xff".to_vec()))
        .env_clear()
        .env(
            OsString::from_vec(b"KEY_\xff".to_vec()),
            OsString::from_vec(b"value-\xff".to_vec()),
        )
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"pipe\0bytes\xff\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"pipe\0bytes\xff\n");
    assert_eq!(output.stderr, b"arguments=2 environment=1\n");
    std::fs::remove_dir(&cwd).unwrap();
    std::fs::remove_dir(root).unwrap();
}
