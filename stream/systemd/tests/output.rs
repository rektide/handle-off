use handle_off_stream_journal::*;
use std::{collections::BTreeMap, io::Write, path::PathBuf};

fn source() -> Source {
    Source {
        backend: Backend::Fixture {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/journal.jsonl"),
        },
        machine_id: "a".repeat(32),
        unit: "worker.service".into(),
        unit_field: UnitField::SystemdUnit,
        selection: Selection::FollowUnit,
        matches: BTreeMap::new(),
    }
}

fn first(state: &Checkpoint) -> Event {
    let mut result = None;
    read(
        state,
        Limits {
            entries: 1,
            ..Limits::default()
        },
        |event| {
            result = Some(event);
            Ok(())
        },
    )
    .unwrap();
    result.unwrap()
}

struct FailOutput {
    bytes: Vec<u8>,
    fail_flush: bool,
}

impl Write for FailOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.fail_flush {
            self.bytes.extend_from_slice(bytes);
            return Ok(bytes.len());
        }
        if !self.bytes.is_empty() {
            return Err(std::io::Error::other("fixture partial write failure"));
        }
        let count = bytes.len().min(7);
        self.bytes.extend_from_slice(&bytes[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        assert!(self.fail_flush, "no flush after failed write");
        Err(std::io::Error::other("fixture flush failure"))
    }
}

#[test]
fn partial_write_and_flush_failures_preserve_disk_and_memory_then_replay() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("state");
    let _lock = CheckpointLock::acquire(&path)?;
    let mut state = Checkpoint::new(source())?;
    state.save_new(&path)?;
    let saved = std::fs::read(&path)?;
    let event = first(&state);
    for fail_flush in [false, true] {
        let mut output = FailOutput { bytes: Vec::new(), fail_flush };
        let error = state.emit_into(&event, &mut output, &path).unwrap_err();
        assert!(error.to_string().contains("checkpoint unchanged"));
        assert!(!output.bytes.is_empty());
        assert_eq!(std::fs::read(&path)?, saved);
        assert!(state.cursor.is_none());
        assert_eq!(state.emitted, 0);
    }
    let mut output = Vec::new();
    assert_eq!(state.emit_into(&event, &mut output, &path)?, Observation::First);
    let line: serde_json::Value = serde_json::from_slice(&output)?;
    assert_eq!(line["event"]["__CURSOR"], "fixture:1");
    assert_eq!(state.cursor.as_deref(), Some("fixture:1"));
    assert_eq!(Checkpoint::load(&path)?.emitted, 1);
    Ok(())
}

#[test]
fn failed_save_after_flush_retains_old_checkpoint_and_replays_exact_output() -> anyhow::Result<()> {
    struct MoveOnFlush {
        from: PathBuf,
        to: PathBuf,
        bytes: Vec<u8>,
    }
    impl Write for MoveOnFlush {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            std::fs::rename(&self.from, &self.to)
        }
    }
    let dir = tempfile::tempdir()?;
    let original = dir.path().join("original");
    let moved = dir.path().join("moved");
    std::fs::create_dir(&original)?;
    let path = original.join("state");
    let lock = CheckpointLock::acquire(&path)?;
    let mut state = Checkpoint::new(source())?;
    state.save_new(&path)?;
    let before = std::fs::read(&path)?;
    let event = first(&state);
    let mut output = MoveOnFlush {
        from: original,
        to: moved.clone(),
        bytes: Vec::new(),
    };
    let error = state.emit_into(&event, &mut output, &path).unwrap_err();
    assert!(error.to_string().contains("output flushed but checkpoint save failed"));
    assert_eq!(state.emitted, 0);
    assert_eq!(std::fs::read(moved.join("state"))?, before);
    drop(lock);
    let path = moved.join("state");
    let _lock = CheckpointLock::acquire(&path)?;
    let mut reopened = Checkpoint::load(&path)?;
    let mut replay = Vec::new();
    reopened.emit_into(&first(&reopened), &mut replay, &path)?;
    assert_eq!(replay, output.bytes);
    assert_eq!(Checkpoint::load(&path)?.cursor.as_deref(), Some("fixture:1"));
    Ok(())
}

#[test]
fn oversized_checkpoint_is_preflighted_before_output_or_replacement() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("state");
    let mut state = Checkpoint::new(source())?;
    state.save_new(&path)?;
    let before = std::fs::read(&path)?;
    let mut event = first(&state);
    event.fields.insert("__CURSOR".into(), "x".repeat(MAX_STATE_BYTES).into());
    let mut output = Vec::new();
    assert!(state.emit_into(&event, &mut output, &path).is_err());
    assert!(output.is_empty());
    assert!(state.cursor.is_none());
    assert_eq!(std::fs::read(&path)?, before);
    state.source.unit = "x".repeat(MAX_STATE_BYTES);
    assert!(state.save(&path).is_err());
    assert_eq!(std::fs::read(&path)?, before);
    Ok(())
}

#[test]
fn control_documents_are_bounded_and_invalid_state_is_not_repaired() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("state");
    let state = Checkpoint::new(source())?;
    for bytes in [vec![b' '; MAX_STATE_BYTES + 1], b"{truncated".to_vec()] {
        std::fs::write(&path, &bytes)?;
        assert!(Checkpoint::load(&path).is_err());
        assert!(Source::load(&path).is_err());
        assert_eq!(std::fs::read(&path)?, bytes);
    }
    for mutation in ["version", "unknown", "cursor"] {
        let mut value = serde_json::to_value(&state)?;
        match mutation {
            "version" => value["version"] = 2.into(),
            "unknown" => value["unknown"] = true.into(),
            "cursor" => value["cursor"] = "".into(),
            _ => unreachable!(),
        }
        let bytes = serde_json::to_vec(&value)?;
        std::fs::write(&path, &bytes)?;
        assert!(Checkpoint::load(&path).is_err());
        assert_eq!(std::fs::read(&path)?, bytes);
    }
    assert!(
        read(
            &state,
            Limits {
                line_bytes: usize::MAX,
                ..Limits::default()
            },
            |_| panic!("invalid limits must fail before source access")
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn repeated_identity_is_not_collapsed_and_ambiguous_cursor_never_emits() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("state");
    let mut state = Checkpoint::new(source())?;
    state.save_new(&path)?;
    let mut event = first(&state);
    event
        .fields
        .insert("_SYSTEMD_INVOCATION_ID".into(), serde_json::json!(["1".repeat(32), "2".repeat(32)]));
    event.fields.insert("MESSAGE".into(), "".into());
    event.fields.insert("TAG".into(), serde_json::json!(["", [0, 255], "tail"]));
    let mut output = Vec::new();
    state.emit_into(&event, &mut output, &path)?;
    let row: serde_json::Value = serde_json::from_slice(&output)?;
    assert!(row["identity"]["invocation"].is_null());
    assert_eq!(row["event"], serde_json::to_value(&event)?);
    assert_eq!(observe(state.identity.as_ref(), &event.identity(&state.source)), Observation::Unknown);
    let before = std::fs::read(&path)?;
    output.clear();
    event.fields.insert("__CURSOR".into(), serde_json::json!(["one", "two"]));
    assert!(state.emit_into(&event, &mut output, &path).is_err());
    assert!(output.is_empty());
    assert_eq!(std::fs::read(&path)?, before);
    Ok(())
}
