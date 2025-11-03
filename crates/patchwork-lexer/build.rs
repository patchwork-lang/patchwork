use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let lexer_path = PathBuf::from("lexer.alex");

    // Generate lexer code from ALEX specification
    parlex_gen::alex::generate(&lexer_path, &out_dir, "lexer", false)
        .expect("Failed to generate lexer");

    // Tell cargo to re-run if the lexer specification changes
    println!("cargo:rerun-if-changed=lexer.alex");
}
