use clap::{Arg, Command};
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};

fn main() -> Result<()> {
    let matches = Command::new("Rusteria - Compiler / Editor and Packaging Tool for Rusterix")
        .version("0.1.0")
        .author("Markus Moenig <markus@moenig.io>")
        .about("Rusteria - Compiler / Editor and Packaging Tool for Rusterix")
        .arg(
            Arg::new("interactive")
                .short('i')
                .long("interactive")
                .help("Enters interactive editing mode.")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("package")
                .short('p')
                .long("package")
                .help("Package the game.")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    if matches.get_flag("interactive") {
        println!("Interactive mode enabled.");
        let mut rl = DefaultEditor::new()?;
        if rl.load_history("history.txt").is_err() {
            println!("No previous history.");
        }
        loop {
            let readline = rl.readline(">> ");
            match readline {
                Ok(line) => {
                    if line == "exit" {
                        println!("Exiting...");
                        break;
                    }
                    rl.add_history_entry(line.as_str())?;
                    println!("Line: {}", line);
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Exiting...");
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!("Exiting...");
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
        rl.save_history("history.txt")?;
    }

    if let Ok(string) = std::fs::read_to_string("world.rxm") {
        println!("string {}", string);
    }

    Ok(())
}
