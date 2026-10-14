#[cfg(unix)]
fn main() -> std::io::Result<()> {
    use handle_off::{invocation::Invocation, stdio::BorrowedStdio};
    use std::{
        fs::File,
        io::{self, Write},
    };

    fn handle(context: &Invocation, stdio: BorrowedStdio<'_>) -> io::Result<()> {
        // Owned duplicates allow safe File I/O without taking the caller's handles.
        let stdio = stdio.try_clone_to_owned()?;
        let mut input = File::from(stdio.stdin);
        let mut output = File::from(stdio.stdout);
        let mut errors = File::from(stdio.stderr);
        io::copy(&mut input, &mut output)?;
        writeln!(
            errors,
            "arguments={} environment={}",
            context.argv.len(),
            context.environment.len()
        )?;
        Ok(())
    }
    let context = Invocation::capture_current()?;
    let (input, output, errors) = (io::stdin(), io::stdout(), io::stderr());
    // No buffered stdio I/O precedes descriptor I/O in this example.
    handle(&context, BorrowedStdio::new(&input, &output, &errors))
}

#[cfg(not(unix))]
fn main() {
    eprintln!("this descriptor example requires Unix");
}
