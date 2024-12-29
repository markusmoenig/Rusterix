pub mod tool;
pub mod window;

use crate::tool::Tool;

use notify::{Event, RecursiveMode, Watcher};
use rusterix::prelude::*;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, LazyLock, Mutex, OnceLock};
use theframework::*;
use vek::Vec2;

#[derive(Debug, Clone)]
#[allow(dead_code, clippy::large_enum_variant)]
enum Cmd {
    OpenWindow,
    ClosingWindow,
    FocusMap(MapMeta),
    Exit,
    MouseDown(Vec2<f32>),
}

use Cmd::*;

// Global sender and receiver for handling communication from / to the window
static TO_WINDOW_TX: OnceLock<mpsc::Sender<Cmd>> = OnceLock::new();
static TO_WINDOW_RX: OnceLock<Arc<Mutex<mpsc::Receiver<Cmd>>>> = OnceLock::new();
static FROM_WINDOW_TX: OnceLock<mpsc::Sender<Cmd>> = OnceLock::new();
static FROM_WINDOW_RX: OnceLock<Arc<Mutex<mpsc::Receiver<Cmd>>>> = OnceLock::new();

static PROMPT: LazyLock<Mutex<String>> = LazyLock::new(|| Mutex::new(String::from("> ")));

fn main() -> Result<()> {
    let mut window_is_open = false;

    // The pipe for the file watcher
    let (watcher_tx, watcher_rx) = mpsc::channel::<notify::Result<Event>>();

    // The local terminal / main thread pipe
    let (tx, rx) = mpsc::channel::<Cmd>();
    let tx_clone = tx.clone();

    // The global pipe between window and command thread
    let (wtx, wrx) = mpsc::channel::<Cmd>();
    TO_WINDOW_TX
        .set(wtx.clone())
        .expect("Failed to set TO_WINDOW_TX");
    TO_WINDOW_RX
        .set(Arc::new(Mutex::new(wrx)))
        .expect("Failed to set TO_WINDOW_RX");

    let (wtx, wrx) = mpsc::channel::<Cmd>();
    FROM_WINDOW_TX
        .set(wtx.clone())
        .expect("Failed to set TO_WINDOW_TX");
    FROM_WINDOW_RX
        .set(Arc::new(Mutex::new(wrx)))
        .expect("Failed to set TO_WINDOW_RX");

    let mut curr_watched_path: Option<PathBuf> = None;

    // Terminal thread handles rustyline input
    let _terminal_thread = std::thread::spawn(move || {
        let mut watcher = notify::recommended_watcher(watcher_tx).ok().unwrap();

        println!("Rusteria. Interactive Rusterix shell.");

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
                        if let Some(tx) = FROM_WINDOW_TX.get() {
                            tx.send(Exit).unwrap();
                        }
                        if let Some(tx) = TO_WINDOW_TX.get() {
                            tx.send(Exit).unwrap();
                        }
                        break;
                    }
                    if line == "open" {
                        window_is_open = true;
                        tx.send(Cmd::OpenWindow).unwrap();
                    }
                    if line.starts_with("focus ") {
                        if !window_is_open {
                            window_is_open = true;
                            tx.send(Cmd::OpenWindow).unwrap();
                        }

                        if let Some(file_name) = line.strip_prefix("focus ") {
                            if let Some(extension) = Path::new(file_name).extension() {
                                match extension.to_str() {
                                    Some("rxm") => {
                                        if watcher
                                            .watch(
                                                std::path::Path::new(file_name),
                                                RecursiveMode::Recursive,
                                            )
                                            .is_ok()
                                        {
                                            if let Some(watched_path) = curr_watched_path {
                                                _ = watcher.unwatch(Path::new(&watched_path));
                                            }
                                            curr_watched_path = Some(PathBuf::from(file_name));
                                        }
                                        if let Some(map) = compile_map(file_name) {
                                            if let Some(tx) = TO_WINDOW_TX.get() {
                                                tx.send(FocusMap(map)).unwrap();
                                            }
                                        }
                                    }
                                    _ => println!("Unknown file type"),
                                }
                            }
                        }
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
        if let Some(rx) = FROM_WINDOW_RX.get() {
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
        while let Ok(watcher_cmd) = watcher_rx.try_recv() {
            match watcher_cmd {
                Ok(event) => {
                    // println!("event: {:?}", event)
                    for path in event.paths {
                        if let Some(extension) = Path::new(&path).extension() {
                            match extension.to_str() {
                                Some("rxm") => {
                                    if let Some(file_name) = path.to_str() {
                                        if let Some(map) = compile_map(file_name) {
                                            if let Some(tx) = TO_WINDOW_TX.get() {
                                                tx.send(FocusMap(map)).unwrap();
                                            }
                                        }
                                    }
                                }
                                _ => println!("Unknown file type"),
                            }
                        }
                    }
                }
                Err(e) => println!("watch error: {:?}", e),
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    });

    // Main thread handles opening / closing of the editor window.
    loop {
        if let Ok(command) = rx.recv() {
            match command {
                OpenWindow => {
                    let cube = crate::window::Editor::new();
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

    Ok(())
}

fn compile_map(file_name: &str) -> Option<MapMeta> {
    let mut mapscript = MapScript::new();
    mapscript.load_map(file_name);
    let result = mapscript.transform(None, None, None);

    match result {
        Ok(meta) => {
            println!("{}", meta.map.info());
            Some(meta)
        }
        Err(errors) => {
            for err in errors {
                println!("{}", err);
            }
            None
        }
    }
}
