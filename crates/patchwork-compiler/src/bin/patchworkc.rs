/// Patchwork compiler CLI

use std::path::PathBuf;
use std::process;
use clap::Parser;
use patchwork_compiler::{Compiler, CompileOptions};

#[derive(Parser, Debug)]
#[command(name = "patchworkc")]
#[command(about = "Patchwork compiler - transforms Patchwork source into executable agent systems")]
#[command(version)]
struct Args {
    /// Input Patchwork source file
    #[arg(value_name = "FILE")]
    input: PathBuf,

    /// Output directory for generated files
    #[arg(short, long, value_name = "DIR")]
    output: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Print the AST and exit (debug mode)
    #[arg(long)]
    dump_ast: bool,
}

fn main() {
    let args = Args::parse();

    // Build compiler options
    let mut options = CompileOptions::new(args.input);

    if let Some(output) = args.output {
        options = options.output_dir(output);
    }

    options = options.verbose(args.verbose);

    // Create and run compiler
    let compiler = Compiler::new(options);

    match compiler.compile() {
        Ok(output) => {
            if args.dump_ast {
                println!("AST for {}:", output.source_file.display());
                println!("{:#?}", output.ast);
            } else if args.verbose {
                println!("Compilation successful!");
                println!("  Source: {}", output.source_file.display());
                println!("  Items: {}", output.ast.items.len());
            }
        }
        Err(e) => {
            eprintln!("Compilation failed: {}", e);
            process::exit(1);
        }
    }
}
