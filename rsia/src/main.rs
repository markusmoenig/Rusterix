use clap::{Command, arg};
use rusteria::{RenderBuffer, Rusteria};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn cli() -> Command {
    Command::new("shpz")
        .about("Denrim Command Line Interface.")
        .author("Markus Moenig")
        .version("0.1.0")
        .allow_external_subcommands(true)
        .arg(arg!([FILE] "Input '.denscr' file").default_value("test.rsia"))
        .arg(
            arg!(-r --resolution <RES> "Output resolution (WIDTHxHEIGHT)").default_value("800x800"),
        )
}

fn main() {
    let matches = cli().get_matches();

    let (width, height) = matches
        .get_one::<String>("resolution")
        .and_then(|r| {
            r.split_once('x')
                .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        })
        .unwrap_or((800, 800));

    let path = PathBuf::from(matches.get_one::<String>("FILE").unwrap());

    let mut ds = Rusteria::default();

    let _module = match ds.parse(path.clone()) {
        Ok(module) => match ds.compile(&module) {
            Ok(()) => {
                println!("Module '{}' compiled successfully.", module.name);
            }
            Err(e) => {
                eprintln!("Error compiling module: {e}");
                return;
            }
        },
        Err(e) => {
            eprintln!("Error parsing module: {e}");
            return;
        }
    };

    if let Some(shade_index) = ds.context.program.shade_index {
        let mut buffer = Arc::new(Mutex::new(RenderBuffer::new(width, height)));
        let t0 = ds.get_time();
        ds.shade(&mut buffer, shade_index);
        let t1 = ds.get_time();
        println!("Rendered in {}ms", t1 - t0);
        let mut png_path = path.clone();
        png_path.set_extension("png");
        buffer.lock().unwrap().save(png_path);
    } else {
        let t0 = ds.get_time();
        let rc = ds.execute();
        let t1 = ds.get_time();
        println!("Executed in {}ms", t1 - t0);
        if let Some(rc) = rc {
            println!("Result: {}", rc.to_string());
        }
    }
}
