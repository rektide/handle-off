use handle_off_stream_journal::*;
use serde_json::{Value, json};
use std::{collections::BTreeMap, ffi::OsStr, path::PathBuf, process::Output};

struct Installed {
    dir: tempfile::TempDir,
    binary: PathBuf,
    source_path: PathBuf,
    source: Source,
}

impl Installed {
    fn new() -> anyhow::Result<Self> {
        let dir = tempfile::tempdir()?;
        let binary = dir.path().join("ho-journal");
        std::fs::copy(env!("CARGO_BIN_EXE_ho-journal"), &binary)?;
        let fixture = dir.path().join("input.jsonl");
        std::fs::copy(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/journal.jsonl"), &fixture)?;
        let source = Source {
            backend: Backend::Fixture { path: fixture },
            machine_id: "a".repeat(32),
            unit: "worker.service".into(),
            unit_field: UnitField::SystemdUnit,
            selection: Selection::FollowUnit,
            matches: BTreeMap::new(),
        };
        let source_path = dir.path().join("source.json");
        std::fs::write(&source_path, serde_json::to_vec(&source)?)?;
        Ok(Self {
            dir,
            binary,
            source_path,
            source,
        })
    }

    fn invoke(&self, args: &[&OsStr]) -> anyhow::Result<Output> {
        Ok(std::process::Command::new(&self.binary).args(args).current_dir(self.dir.path()).output()?)
    }

    fn start(&self, path: &std::path::Path, limit: &str) -> anyhow::Result<Output> {
        self.invoke(&[
            "read".as_ref(),
            "--source".as_ref(),
            self.source_path.as_ref(),
            "--checkpoint".as_ref(),
            path.as_ref(),
            "--start".as_ref(),
            "head".as_ref(),
            "--limit".as_ref(),
            limit.as_ref(),
        ])
    }
}

fn rows(output: Output) -> Vec<Value> {
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn installed_process_succession_and_independent_readers_preserve_full_event_identity() -> anyhow::Result<()> {
    let installed = Installed::new()?;
    let a = installed.dir.path().join("reader a #.json");
    let b = installed.dir.path().join("reader-b.json");
    let prefix = rows(installed.start(&a, "2")?);
    assert_eq!(prefix.len(), 2);
    assert_eq!(prefix[0]["observation"], "first");
    assert_eq!(prefix[1]["observation"], "same_invocation");
    assert_ne!(prefix[0]["identity"]["pid"], prefix[1]["identity"]["pid"]);
    assert_eq!(prefix[1]["event"]["MESSAGE"], json!([0, 255, 10]));
    assert_eq!(prefix[1]["event"]["TAG"], json!(["one", "two"]));
    let reference = installed.invoke(&["reference".as_ref(), "--checkpoint".as_ref(), a.as_ref()])?;
    assert!(reference.status.success());
    let reference = String::from_utf8(reference.stdout)?.trim().to_owned();
    assert!(reference.contains("%23"));
    let suffix = rows(installed.invoke(&["resume".as_ref(), "--checkpoint".as_ref(), reference.as_ref()])?);
    assert_eq!(
        suffix.iter().map(|row| row["observation"].as_str().unwrap()).collect::<Vec<_>>(),
        ["invocation_changed", "boot_changed", "unknown"]
    );
    assert_eq!(suffix[0]["identity"]["pid"], prefix[0]["identity"]["pid"]);
    assert_ne!(suffix[0]["identity"]["invocation"], prefix[0]["identity"]["invocation"]);
    assert_ne!(suffix[0]["identity"]["boot_id"], suffix[1]["identity"]["boot_id"]);
    assert!(suffix[2]["identity"]["invocation"].is_null());
    let all = rows(installed.start(&b, "100")?);
    let combined: Vec<_> = prefix.into_iter().chain(suffix).collect();
    assert_eq!(combined, all);
    let Backend::Fixture { path } = &installed.source.backend else {
        unreachable!()
    };
    let expected: Vec<Value> = std::fs::read_to_string(path)?
        .lines()
        .map(|line| serde_json::from_str::<Event>(line).unwrap())
        .filter(|event| installed.source.accepts(event))
        .map(|event| serde_json::to_value(event).unwrap())
        .collect();
    assert_eq!(all.iter().map(|row| row["event"].clone()).collect::<Vec<_>>(), expected);
    assert_eq!(Checkpoint::load(&a)?.emitted, 5);
    assert_eq!(Checkpoint::load(&b)?.cursor, Checkpoint::load(&a)?.cursor);
    assert!(rows(installed.invoke(&["resume".as_ref(), "--checkpoint".as_ref(), reference.as_ref()])?).is_empty());
    Ok(())
}

#[test]
fn source_mismatches_fail_before_reading_even_when_bound_source_is_unavailable() -> anyhow::Result<()> {
    let installed = Installed::new()?;
    let path = installed.dir.path().join("state");
    Checkpoint::new(installed.source.clone())?.save_new(&path)?;
    let before = std::fs::read(&path)?;
    let Backend::Fixture { path: fixture } = &installed.source.backend else {
        unreachable!()
    };
    std::fs::remove_file(fixture)?;
    let mut variants = vec![installed.source.clone(); 6];
    variants[0].unit = "other.service".into();
    variants[1].machine_id = "b".repeat(32);
    variants[2].matches.insert("_BOOT_ID".into(), "c".repeat(32));
    variants[3].selection = Selection::Invocation { id: "1".repeat(32) };
    variants[4].unit_field = UnitField::ManagerTarget;
    variants[5].backend = Backend::Fixture {
        path: installed.dir.path().join("other-absent.jsonl"),
    };
    for source in variants {
        std::fs::write(&installed.source_path, serde_json::to_vec(&source)?)?;
        let output = installed.invoke(&[
            "resume".as_ref(),
            "--checkpoint".as_ref(),
            path.as_ref(),
            "--source".as_ref(),
            installed.source_path.as_ref(),
        ])?;
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("source binding mismatch"));
        assert!(output.stdout.is_empty());
        assert_eq!(std::fs::read(&path)?, before);
    }
    Ok(())
}

#[test]
fn missing_and_alternate_fixture_cursors_fail_without_output_or_state_change() -> anyhow::Result<()> {
    let installed = Installed::new()?;
    let path = installed.dir.path().join("state");
    let mut state = Checkpoint::new(installed.source.clone())?;
    state.identity = Some(Identity::default());
    for cursor in ["missing", "fixture:1;alternate-spelling"] {
        state.cursor = Some(cursor.into());
        state.save(&path)?;
        let before = std::fs::read(&path)?;
        let output = installed.invoke(&["resume".as_ref(), "--checkpoint".as_ref(), path.as_ref()])?;
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("cursor unavailable"));
        assert!(output.stdout.is_empty());
        assert_eq!(std::fs::read(&path)?, before);
    }
    Ok(())
}

#[test]
fn describe_is_offline_and_checkpoint_uris_are_local_unambiguous_references() -> anyhow::Result<()> {
    let installed = Installed::new()?;
    let mut source = installed.source.clone();
    source.backend = Backend::Journal {
        scope: Scope::User,
        directory: Some(installed.dir.path().join("nonexistent-journal")),
    };
    source.unit_field = UnitField::UserManagerTarget;
    source.selection = Selection::Invocation { id: "2".repeat(32) };
    std::fs::write(&installed.source_path, serde_json::to_vec(&source)?)?;
    let before = std::fs::read_dir(installed.dir.path())?.count();
    let output = installed.invoke(&["describe".as_ref(), "--source".as_ref(), installed.source_path.as_ref()])?;
    assert!(output.status.success());
    let description: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(description["backend_profile"], "journalctl-json-v1");
    assert_eq!(description["selectors"]["USER_INVOCATION_ID"], "2".repeat(32));
    assert_eq!(description["selectors"]["USER_UNIT"], "worker.service");
    assert!(description["selectors"].get("UNIT").is_none());
    assert_eq!(description["cursor_comparison"], "exact-string-only");
    assert_eq!(description["source"], serde_json::to_value(source)?);
    assert_eq!(std::fs::read_dir(installed.dir.path())?.count(), before);
    for value in ["file:relative", "file://other-host/state", "file:///state#fragment", "file:///state?query"] {
        assert!(checkpoint_path(OsStr::new(value)).is_err(), "{value}");
    }
    assert_eq!(checkpoint_path(OsStr::new("local # file"))?, PathBuf::from("local # file"));
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let native = OsStr::from_bytes(b"native-\xff");
        assert_eq!(checkpoint_path(native)?.as_os_str(), native);
        let path = installed.dir.path().join(native);
        std::fs::write(&path, b"reference target")?;
        let uri = reference(&path)?;
        assert!(uri.contains("%FF"));
        assert_eq!(checkpoint_path(OsStr::new(&uri))?, path.canonicalize()?);
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn journalctl_nearest_neighbor_is_refused_before_emitting_any_entry() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let installed = Installed::new()?;
    let executable = installed.dir.path().join("journalctl");
    std::fs::write(
        &executable,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$TEST_JOURNAL_ARGV\"\n/bin/cat \"$TEST_JOURNAL_ROWS\"\n",
    )?;
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))?;
    let mut source = installed.source.clone();
    let Backend::Fixture { path: fixture } = &source.backend else {
        unreachable!()
    };
    let fixture = fixture.clone();
    source.backend = Backend::Journal {
        scope: Scope::System,
        directory: None,
    };
    let mut state = Checkpoint::new(source)?;
    state.cursor = Some("saved-but-unavailable".into());
    state.identity = Some(Identity::default());
    let path = installed.dir.path().join("state");
    let argv = installed.dir.path().join("journalctl-argv");
    state.save_new(&path)?;
    let before = std::fs::read(&path)?;
    let output = std::process::Command::new(&installed.binary)
        .args(["resume", "--checkpoint"])
        .arg(&path)
        .env("PATH", installed.dir.path())
        .env("TEST_JOURNAL_ROWS", fixture)
        .env("TEST_JOURNAL_ARGV", &argv)
        .output()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("refusing nearest-entry resume"));
    assert!(output.stdout.is_empty());
    assert_eq!(std::fs::read(&path)?, before);
    let argv = std::fs::read_to_string(argv)?;
    assert!(argv.lines().any(|arg| arg == "--cursor=saved-but-unavailable"));
    assert!(argv.lines().any(|arg| arg == "--lines=all"));
    assert!(!argv.lines().any(|arg| arg == "--after-cursor" || arg == "--no-tail"));
    Ok(())
}
