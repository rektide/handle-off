use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use handle_off_stream_journal::{Checkpoint, CheckpointLock, InitialPosition, Limits, Source, checkpoint_path, read, reference};
use std::{ffi::OsString, io, path::PathBuf};

#[derive(Parser)]
#[command(version, about = "Experimental source-bound journal checkpoint utilities")]
struct Cli {
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Start at the explicit head/tail policy, requiring a new checkpoint file.
    Read {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        checkpoint: OsString,
        #[arg(long)]
        start: InitialPosition,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Resume strictly after the saved entry. Optionally assert an exact source binding.
    Resume {
        #[arg(long)]
        checkpoint: OsString,
        #[arg(long)]
        source: Option<PathBuf>,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Print validated saved state without querying the journal.
    Status {
        #[arg(long)]
        checkpoint: OsString,
    },
    /// Print the checkpoint's file URI (not the opaque journal cursor).
    Reference {
        #[arg(long)]
        checkpoint: OsString,
    },
    /// Describe explicit source routing without opening the fixture, journal or checkpoint.
    Describe {
        #[arg(long)]
        source: PathBuf,
    },
}

fn run_batch(mut state: Checkpoint, path: PathBuf, limit: usize) -> Result<()> {
    let snapshot = state.clone();
    let mut output = io::stdout().lock();
    read(
        &snapshot,
        Limits {
            entries: limit,
            ..Limits::default()
        },
        |event| {
            state.emit_into(&event, &mut output, &path)?;
            Ok(())
        },
    )?;
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Action::Read {
            source: input,
            checkpoint,
            start,
            limit,
        } => {
            ensure!(limit > 0, "limit must be positive");
            let checkpoint = checkpoint_path(&checkpoint)?;
            let _lock = CheckpointLock::acquire(&checkpoint)?;
            ensure!(!checkpoint.try_exists()?, "checkpoint exists; use resume or a new path");
            let state = Checkpoint::new_at(Source::load(&input)?, start)?;
            state.save_new(&checkpoint)?;
            run_batch(state, checkpoint, limit)
        }
        Action::Resume {
            checkpoint,
            source: input,
            limit,
        } => {
            let checkpoint = checkpoint_path(&checkpoint)?;
            let _lock = CheckpointLock::acquire(&checkpoint)?;
            let state = Checkpoint::load(&checkpoint)?;
            if let Some(input) = input {
                ensure!(state.source == Source::load(&input)?, "source binding mismatch");
            }
            run_batch(state, checkpoint, limit)
        }
        Action::Status { checkpoint } => {
            let checkpoint = checkpoint_path(&checkpoint)?;
            println!("{}", serde_json::to_string_pretty(&Checkpoint::load(&checkpoint)?)?);
            Ok(())
        }
        Action::Reference { checkpoint } => {
            let checkpoint = checkpoint_path(&checkpoint)?;
            Checkpoint::load(&checkpoint)?;
            println!("{}", reference(&checkpoint)?);
            Ok(())
        }
        Action::Describe { source } => {
            println!("{}", serde_json::to_string_pretty(&Source::load(&source)?.describe()?)?);
            Ok(())
        }
    }
}
