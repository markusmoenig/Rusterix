use clap::{Command, arg};
use rusteria::Rusteria;
use std::path::PathBuf;

fn cli() -> Command {
    Command::new("shpz")
        .about("Denrim Command Line Interface.")
        .author("Markus Moenig")
        .version("0.1.0")
        .allow_external_subcommands(true)
        .arg(arg!([FILE] "Input '.denscr' file").default_value("test.rsta"))
        .arg(
            arg!(-r --resolution <RES> "Output resolution (WIDTHxHEIGHT)").default_value("800x800"),
        )
}

fn main() {
    let matches = cli().get_matches();

    // let (width, height) = matches
    //     .get_one::<String>("resolution")
    //     .and_then(|r| {
    //         r.split_once('x')
    //             .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
    //     })
    //     .unwrap_or((800, 800));

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

    let t0 = ds.get_time();
    let rc = ds.execute();
    let t1 = ds.get_time();

    println!("Executed in {:.2}s", (t1 - t0) as f32 / 1000.0);
    if let Some(rc) = rc {
        println!("Result: {}", rc.to_string());
    }
}
