//! Explicit invocation context. Capturing never changes process cwd/environment.
use std::{ffi::OsString, io, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    /// Includes argv[0] when present. An explicit invocation may have no arguments.
    pub argv: Vec<OsString>,
    /// A full environment snapshot, not an overlay. May contain credentials.
    pub environment: Vec<(OsString, OsString)>,
    pub cwd: PathBuf,
}
impl Invocation {
    /// Snapshot only: concurrent external environment mutation is not made atomic.
    /// Native OsStrings preserve non-UTF-8 Unix bytes. No parsing or execution occurs.
    pub fn capture_current() -> io::Result<Self> {
        Ok(Self {
            argv: std::env::args_os().collect(),
            environment: std::env::vars_os().collect(),
            cwd: std::env::current_dir()?,
        })
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    #[test]
    fn native_values_remain_lossless() {
        let value = OsString::from_vec(vec![b'x', 0xff]);
        let invocation = Invocation {
            argv: vec![value.clone()],
            environment: vec![(value.clone(), value.clone())],
            cwd: PathBuf::from(value),
        };
        let copy = invocation.clone();
        assert_eq!(copy.argv[0].as_bytes(), b"x\xff");
        assert_eq!(copy.environment[0].0.as_bytes(), b"x\xff");
        assert_eq!(copy.environment[0].1.as_bytes(), b"x\xff");
        assert_eq!(copy.cwd.as_os_str().as_bytes(), b"x\xff");
    }

    #[test]
    fn capture_child() {
        if std::env::var_os("HANDLE_OFF_CAPTURE_CHILD").is_none() {
            return;
        }
        let invocation = Invocation::capture_current().unwrap();
        assert_eq!(
            invocation
                .environment
                .iter()
                .find(|(key, _)| key.as_bytes() == b"KEY_\xff")
                .unwrap()
                .1
                .as_bytes(),
            b"value-\xff"
        );
        assert_eq!(invocation.cwd.file_name().unwrap().as_bytes(), b"cwd-\xff");
        assert_eq!(std::env::current_dir().unwrap(), invocation.cwd);
    }

    #[test]
    fn capture_preserves_non_utf8_process_context() {
        let root = std::env::temp_dir().join(format!("handle-off-capture-{}", std::process::id()));
        let cwd = root.join(OsString::from_vec(b"cwd-\xff".to_vec()));
        std::fs::create_dir_all(&cwd).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "tests::capture_child"])
            .env_clear()
            .env("HANDLE_OFF_CAPTURE_CHILD", "1")
            .env(
                OsString::from_vec(b"KEY_\xff".to_vec()),
                OsString::from_vec(b"value-\xff".to_vec()),
            )
            .current_dir(&cwd)
            .output()
            .unwrap();
        std::fs::remove_dir(cwd).unwrap();
        std::fs::remove_dir(root).unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
}
