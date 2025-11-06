use patchwork_parser::{parse, ast_dump::dump_program};
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <file.pw>", args[0]);
        eprintln!();
        eprintln!("Parse a patchwork file and dump its AST structure");
        process::exit(1);
    }

    let filename = &args[1];

    // Read file
    let input = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };

    // Parse
    let program = match parse(&input) {
        Ok(prog) => prog,
        Err(e) => {
            eprintln!("Parse error in '{}':", filename);
            eprintln!("{:?}", e);
            process::exit(1);
        }
    };

    // Dump AST
    let dump = dump_program(&program);
    println!("{}", dump);
}
