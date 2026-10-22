use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use handle_off_stream_journal::{Checkpoint, CheckpointLock, InitialPosition, Limits, Source, read, reference};
use std::{
    fs::File,
    io::{self, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(version, about = "Experimental source-bound journal checkpoint utilities")]
struct Cli {
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Start from the oldest selected entry, requiring a new checkpoint file.
    Read {
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        start: InitialPosition,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Resume strictly after the saved entry. Optionally assert an exact source binding.
    Resume {
        #[arg(long)]
        checkpoint: PathBuf,
        #[arg(long)]
        source: Option<PathBuf>,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Print validated saved state without querying the journal.
    Status {
        #[arg(long)]
        checkpoint: PathBuf,
    },
    /// Print the checkpoint's file URI (not the opaque journal cursor).
    Reference {
        #[arg(long)]
        checkpoint: PathBuf,
    },
}

fn source(path: PathBuf) -> Result<Source> {
    Ok(serde_json::from_reader(File::open(path)?)?)
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
            let observation = state.advance(&event)?;
            serde_json::to_writer(
                &mut output,
                &serde_json::json!({"observation": observation, "identity": state.identity, "event": event}),
            )?;
            output.write_all(b"\n")?;
            output.flush()?;
            state.save(&path)?;
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
            let _lock = CheckpointLock::acquire(&checkpoint)?;
            ensure!(!checkpoint.try_exists()?, "checkpoint exists; use resume or a new path");
            let state = Checkpoint::new_at(source(input)?, start)?;
            state.save_new(&checkpoint)?;
            run_batch(state, checkpoint, limit)
        }
        Action::Resume {
            checkpoint,
            source: input,
            limit,
        } => {
            let _lock = CheckpointLock::acquire(&checkpoint)?;
            let state = Checkpoint::load(&checkpoint)?;
            if let Some(input) = input {
                ensure!(state.source == source(input)?, "source binding mismatch");
            }
            run_batch(state, checkpoint, limit)
        }
        Action::Status { checkpoint } => {
            println!("{}", serde_json::to_string_pretty(&Checkpoint::load(&checkpoint)?)?);
            Ok(())
        }
        Action::Reference { checkpoint } => {
            Checkpoint::load(&checkpoint)?;
            println!("{}", reference(&checkpoint)?);
            Ok(())
        }
    }
}
