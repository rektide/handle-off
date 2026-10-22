use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use handle_off_stream_durable::{Binding, FileReader, HttpStream};
use std::{
    io::{self, Read, Write},
    path::PathBuf,
};

#[derive(Parser)]
#[command(version, about = "Pre-alpha Durable Streams HTTP workbench; finite reads, explicit file checkpoints")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Args)]
struct Source {
    /// Full HTTP stream URL (no query, fragment or credentials).
    url: String,
    #[arg(long, default_value = "application/octet-stream")]
    content_type: String,
}

#[derive(Args)]
struct Reader {
    #[command(flatten)]
    source: Source,
    #[arg(long)]
    state: PathBuf,
    /// Operator-declared stream generation; the server does not verify it.
    #[arg(long)]
    incarnation: String,
    /// Maximum complete HTTP response body; oversized responses fail without output.
    #[arg(long, default_value_t = 1048576)]
    max_bytes: u64,
}

impl Reader {
    fn binding(&self) -> Result<Binding> {
        Binding::new(&self.source.url, self.incarnation.clone(), self.source.content_type.clone(), self.max_bytes)
    }
}

#[derive(Subcommand)]
enum Command {
    Create {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        fork_from: Option<String>,
        #[arg(long, requires = "fork_from")]
        fork_offset: Option<String>,
    },
    /// Append bounded stdin once, without automatic retry.
    Append {
        #[command(flatten)]
        source: Source,
        #[arg(long, default_value_t = 1048576)]
        max_bytes: u64,
    },
    Head {
        url: String,
    },
    /// One complete response to stdout, flush, then atomic checkpoint.
    Read {
        #[command(flatten)]
        reader: Reader,
    },
    Status {
        #[arg(long)]
        state: PathBuf,
    },
    /// Explicitly replace a checkpoint binding and opaque offset (default: beginning).
    Reset {
        #[command(flatten)]
        reader: Reader,
        #[arg(long, default_value = "-1", allow_hyphen_values = true)]
        offset: String,
    },
}

fn print_headers(headers: reqwest::header::HeaderMap) -> Result<()> {
    let map: std::collections::BTreeMap<_, _> = headers
        .iter()
        .map(|(k, v)| Ok((k.as_str(), v.to_str()?)))
        .collect::<Result<_, reqwest::header::ToStrError>>()?;
    writeln!(io::stdout().lock(), "{}", serde_json::to_string_pretty(&map)?)?;
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Create {
            source,
            fork_from,
            fork_offset,
        } => print_headers(HttpStream::new(&source.url)?.create(&source.content_type, fork_from.as_deref(), fork_offset.as_deref())?)?,
        Command::Append { source, max_bytes } => {
            anyhow::ensure!(max_bytes > 0 && max_bytes < u64::MAX, "invalid input bound");
            let mut body = Vec::new();
            io::stdin().take(max_bytes + 1).read_to_end(&mut body)?;
            anyhow::ensure!(body.len() as u64 <= max_bytes, "input exceeds bound");
            print_headers(HttpStream::new(&source.url)?.append(&source.content_type, body)?)?;
        }
        Command::Head { url } => print_headers(HttpStream::new(&url)?.head()?)?,
        Command::Read { reader } => {
            let binding = reader.binding()?;
            let batch = FileReader::open(&reader.state)?.read_into(&binding, &mut io::stdout().lock())?;
            eprintln!("bytes={} next_offset={} up_to_date={}", batch.body.len(), batch.next_offset, batch.up_to_date);
        }
        Command::Status { state } => writeln!(io::stdout().lock(), "{}", serde_json::to_string_pretty(&FileReader::open(state)?.status()?)?)?,
        Command::Reset { reader, offset } => {
            let binding = reader.binding()?;
            let state = FileReader::open(&reader.state)?.reset(binding, offset)?;
            writeln!(io::stdout().lock(), "{}", serde_json::to_string_pretty(&state)?)?;
        }
    }
    Ok(())
}
