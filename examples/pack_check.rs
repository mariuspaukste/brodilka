//! Check a world pack against this engine:
//! `cargo run --example pack_check -- path/to/pack.toml`
//! Prints what the engine will ignore; fails on a pack it cannot read.
use brodilka::{data, pack::Pack};

fn main() {
    let mut bad = false;
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: pack_check PACK.toml…");
        std::process::exit(2);
    }
    for path in paths {
        let src = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{path}: {e}");
                bad = true;
                continue;
            }
        };
        match Pack::parse(&src) {
            Err(e) => {
                eprintln!("{path}: {e}");
                bad = true;
            }
            Ok(p) => {
                let w = data::pack_warnings(&p);
                println!(
                    "{path}: \"{}\" ({}), {} text and {} word replacements, {} warning(s)",
                    p.name,
                    p.id,
                    p.text.len(),
                    p.words.len(),
                    w.len()
                );
                for s in &w {
                    println!("  warning: {s}");
                }
                if !w.is_empty() && std::env::var("PACK_STRICT").is_ok() {
                    bad = true;
                }
            }
        }
    }
    if bad {
        std::process::exit(1);
    }
}
