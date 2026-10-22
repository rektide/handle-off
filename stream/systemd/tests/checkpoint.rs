use handle_off_stream_journal::*;
use std::{collections::BTreeMap, path::PathBuf};

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

#[test]
fn resume_preserves_order_raw_values_and_identity_transitions() -> anyhow::Result<()> {
    let mut state = Checkpoint::new(source())?;
    let first = state.clone();
    let mut observations = Vec::new();
    read(
        &first,
        Limits {
            entries: 2,
            ..Limits::default()
        },
        |event| {
            if event.cursor()? == "fixture:2" {
                assert_eq!(event.fields["MESSAGE"], serde_json::json!([0, 255, 10]));
                assert_eq!(event.fields["TAG"], serde_json::json!(["one", "two"]));
            }
            observations.push(state.advance(&event)?);
            Ok(())
        },
    )?;
    assert_eq!(state.cursor.as_deref(), Some("fixture:2"));
    let second = state.clone();
    read(&second, Limits::default(), |event| {
        observations.push(state.advance(&event)?);
        Ok(())
    })?;
    assert_eq!(
        observations,
        [
            Observation::First,
            Observation::SameInvocation,
            Observation::InvocationChanged,
            Observation::BootChanged,
            Observation::Unknown
        ]
    );
    assert_eq!(state.emitted, 5);
    assert_eq!(read(&state, Limits::default(), |_| panic!("at end"))?, 0);
    Ok(())
}

#[test]
fn fixed_invocation_and_task_selection() -> anyhow::Result<()> {
    let mut input = source();
    input.selection = Selection::Invocation { id: "1".repeat(32) };
    input.matches.insert("TASK_ID".into(), "task-a".into());
    assert_eq!(read(&Checkpoint::new(input)?, Limits::default(), |_| Ok(()))?, 2);
    Ok(())
}

#[test]
fn manager_job_identity_keeps_target_and_emitter_incarnations_distinct() -> anyhow::Result<()> {
    let mut input = source();
    input.unit_field = UnitField::ManagerTarget;
    input.selection = Selection::Invocation { id: "1".repeat(32) };
    input.matches.insert("JOB_ID".into(), "7".into());
    let identity_source = input.clone();
    let checkpoint = Checkpoint::new(input)?;
    let mut identity = None;
    assert_eq!(
        read(&checkpoint, Limits::default(), |event| {
            identity = Some(event.identity(&identity_source));
            Ok(())
        })?,
        1
    );
    let identity = identity.expect("selected manager event");
    assert_eq!(identity.unit.as_deref(), Some("worker.service"));
    assert_eq!(identity.emitter_unit.as_deref(), Some("init.scope"));
    assert_eq!(identity.job_id.as_deref(), Some("7"));
    assert_eq!(identity.pid.as_deref(), Some("1"));
    assert_eq!(identity.invocation.unwrap().value, "1".repeat(32));
    Ok(())
}

#[test]
fn manager_invocation_transition_uses_target_cycle_not_emitter_pid_or_id() -> anyhow::Result<()> {
    let mut input = source();
    input.unit_field = UnitField::ManagerTarget;
    let mut state = Checkpoint::new(input)?;
    let snapshot = state.clone();
    let mut observations = Vec::new();
    assert_eq!(
        read(&snapshot, Limits::default(), |event| {
            observations.push(state.advance(&event)?);
            Ok(())
        })?,
        2
    );
    assert_eq!(observations, [Observation::First, Observation::InvocationChanged]);
    assert_eq!(state.identity.as_ref().unwrap().job_id.as_deref(), Some("8"));
    assert_eq!(state.identity.as_ref().unwrap().pid.as_deref(), Some("1"));
    Ok(())
}

#[test]
fn missing_cursor_and_budget_fail_without_callback() -> anyhow::Result<()> {
    let mut state = Checkpoint::new(source())?;
    state.cursor = Some("rotated-away".into());
    state.identity = Some(Identity::default());
    assert!(
        read(&state, Limits::default(), |_| panic!("must not reset"))
            .unwrap_err()
            .to_string()
            .contains("unavailable")
    );
    let fresh = Checkpoint::new(source())?;
    assert!(
        read(
            &fresh,
            Limits {
                line_bytes: 8,
                ..Limits::default()
            },
            |_| panic!("oversize")
        )
        .is_err()
    );
    assert!(
        read(
            &fresh,
            Limits {
                scan_bytes: 8,
                ..Limits::default()
            },
            |_| panic!("scan limit")
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn durable_roundtrip_and_writer_exclusion() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("state with #.json");
    let lock = CheckpointLock::acquire(&path)?;
    assert!(CheckpointLock::acquire(&path).is_err());
    let state = Checkpoint::new(source())?;
    state.save(&path)?;
    assert_eq!(Checkpoint::load(&path)?.source, state.source);
    assert!(reference(&path)?.contains("%23"));
    let before = std::fs::read(&path)?;
    assert!(Checkpoint::new(source())?.save_new(&path).is_err());
    assert_eq!(std::fs::read(&path)?, before);
    let mut invalid = state;
    invalid.version = 2;
    assert!(invalid.save(&path).is_err());
    assert_eq!(std::fs::read(&path)?, before);
    drop(lock);
    CheckpointLock::acquire(&path)?;
    Ok(())
}

#[test]
fn consumer_failure_leaves_last_acknowledged_position() -> anyhow::Result<()> {
    let state = Checkpoint::new(source())?;
    let mut accepted = state.clone();
    let result = read(&state, Limits::default(), |event| {
        if event.cursor()? == "fixture:2" {
            anyhow::bail!("sink unavailable");
        }
        accepted.advance(&event)?;
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(accepted.cursor.as_deref(), Some("fixture:1"));
    let mut next = None;
    read(
        &accepted,
        Limits {
            entries: 1,
            ..Limits::default()
        },
        |event| {
            next = Some(event.cursor()?.to_owned());
            Ok(())
        },
    )?;
    assert_eq!(next.as_deref(), Some("fixture:2"));
    Ok(())
}

#[test]
fn cli_source_mismatch_does_not_modify_checkpoint() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("checkpoint.json");
    Checkpoint::new(source())?.save(&path)?;
    let before = std::fs::read(&path)?;
    let mut other = source();
    other.unit = "other.service".into();
    let source_path = dir.path().join("other.json");
    std::fs::write(&source_path, serde_json::to_vec(&other)?)?;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ho-journal"))
        .arg("resume")
        .arg("--checkpoint")
        .arg(&path)
        .arg("--source")
        .arg(source_path)
        .output()?;
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(std::fs::read(&path)?, before);
    Ok(())
}

#[test]
fn cli_read_tail_resume_status_and_reference_work_end_to_end() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let source_path = dir.path().join("source.json");
    let checkpoint_path = dir.path().join("checkpoint.json");
    std::fs::write(&source_path, serde_json::to_vec(&source())?)?;
    let read_output = std::process::Command::new(env!("CARGO_BIN_EXE_ho-journal"))
        .arg("read")
        .arg("--source")
        .arg(&source_path)
        .arg("--checkpoint")
        .arg(&checkpoint_path)
        .arg("--start")
        .arg("tail")
        .arg("--limit")
        .arg("2")
        .output()?;
    assert!(read_output.status.success(), "{}", String::from_utf8_lossy(&read_output.stderr));
    let rows: Vec<serde_json::Value> = String::from_utf8(read_output.stdout)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["event"]["__CURSOR"], "fixture:4");
    assert_eq!(rows[1]["event"]["__CURSOR"], "fixture:5");
    assert_eq!(Checkpoint::load(&checkpoint_path)?.cursor.as_deref(), Some("fixture:5"));

    let resume = std::process::Command::new(env!("CARGO_BIN_EXE_ho-journal"))
        .arg("resume")
        .arg("--checkpoint")
        .arg(&checkpoint_path)
        .output()?;
    assert!(resume.status.success(), "{}", String::from_utf8_lossy(&resume.stderr));
    assert!(resume.stdout.is_empty());

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_ho-journal"))
        .arg("status")
        .arg("--checkpoint")
        .arg(&checkpoint_path)
        .output()?;
    assert!(status.status.success());
    assert_eq!(serde_json::from_slice::<Checkpoint>(&status.stdout)?.emitted, 2);

    let uri = std::process::Command::new(env!("CARGO_BIN_EXE_ho-journal"))
        .arg("reference")
        .arg("--checkpoint")
        .arg(&checkpoint_path)
        .output()?;
    assert!(uri.status.success());
    assert!(String::from_utf8(uri.stdout)?.starts_with("file://"));
    Ok(())
}
