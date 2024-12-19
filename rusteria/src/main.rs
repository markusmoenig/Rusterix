pub mod window;

use clap::{Arg, Command};
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};
use std::sync::{mpsc, Arc, LazyLock, Mutex, OnceLock};
use theframework::*;
use vek::Vec2;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum Cmd {
    OpenWindow,
    ClosingWindow,
    Exit,
    MouseDown(Vec2<f32>),
}

use Cmd::*;

// Global sender and receiver for handling communication from / to the window
static TX: OnceLock<mpsc::Sender<Cmd>> = OnceLock::new();
static RX: OnceLock<Arc<Mutex<mpsc::Receiver<Cmd>>>> = OnceLock::new();

static PROMPT: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new(String::from("> ")));

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

    // Interactive mode
    if matches.get_flag("interactive") {
        // The local terminal / main thread pipe
        let (tx, rx) = mpsc::channel::<Cmd>();
        let tx_clone = tx.clone();

        // The global pipe between window and command thread
        let (wtx, wrx) = mpsc::channel::<Cmd>();
        TX.set(wtx.clone()).expect("Failed to set TX");
        RX.set(Arc::new(Mutex::new(wrx))).expect("Failed to set RX");

        // Terminal thread handles rustyline input
        let _terminal_thread = std::thread::spawn(move || {
            println!("Interactive mode enabled.");
            let mut rl = DefaultEditor::new().unwrap();
            //rl.set_reader(reader);
            if rl.load_history("history.txt").is_err() {
                println!("No previous history.");
            }
            loop {
                let readline = rl.readline(&PROMPT.lock().unwrap());
                match readline {
                    Ok(line) => {
                        if line == "exit" || line == "quit" {
                            tx.send(Cmd::Exit).unwrap();
                            if let Some(tx) = TX.get() {
                                tx.send(Exit).unwrap();
                            }
                            break;
                        }
                        if line == "open" {
                            tx.send(Cmd::OpenWindow).unwrap();
                        }
                        rl.add_history_entry(line.as_str()).unwrap();
                    }
                    Err(ReadlineError::Interrupted) => {
                        tx.send(Cmd::Exit).unwrap();
                        break;
                    }
                    Err(ReadlineError::Eof) => {
                        tx.send(Cmd::Exit).unwrap();
                        break;
                    }
                    Err(err) => {
                        println!("Error: {:?}", err);
                        tx.send(Cmd::Exit).unwrap();
                        break;
                    }
                }
            }
            rl.save_history("history.txt").unwrap();

            // if let Ok(string) = std::fs::read_to_string("world.rxm") {
            //     println!("string {}", string);
            // }
        });

        // Command thread
        let command_thread = std::thread::spawn(move || loop {
            if let Some(rx) = RX.get() {
                if let Ok(r) = rx.lock() {
                    if let Ok(command) = r.try_recv() {
                        match command {
                            Exit => {
                                break;
                            }
                            ClosingWindow => {
                                tx_clone.send(Cmd::Exit).unwrap();
                                break;
                            }
                            MouseDown(pos) => {
                                println!("mouse {}", pos);
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        // Main thread handles opening / closing of the editor window.
        loop {
            if let Ok(command) = rx.recv() {
                match command {
                    OpenWindow => {
                        let cube = crate::window::Cube::new();
                        let mut app = TheApp::new();
                        let () = app.run(Box::new(cube));
                    }
                    ClosingWindow => {
                        break;
                    }
                    Exit => {
                        break;
                    }
                    _ => {}
                }
            }
        }

        //terminal_thread.join().unwrap();
        command_thread.join().unwrap();

        println!("Exiting...");
    }

    //
    Ok(())
}
