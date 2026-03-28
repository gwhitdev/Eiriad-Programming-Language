use std::env;
use std::fs;
use std::io::{self, Write};

use eiriad::{eval_source, Runtime};

fn main() {
    let mut runtime = Runtime::new();
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let path = &args[1];
        match fs::read_to_string(path) {
            Ok(source) => run_chunk(&mut runtime, &source),
            Err(err) => eprintln!("Failed to read {}: {}", path, err),
        }
        return;
    }

    println!("EIRIAD REPL v0.1");
    println!("Type :quit to exit, :env to inspect bindings, :reset to clear state.");

    let mut input = String::new();
    loop {
        print!("eiriad> ");
        if io::stdout().flush().is_err() {
            eprintln!("Failed to flush stdout");
            break;
        }

        input.clear();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {
                let line = input.trim();
                if line.is_empty() {
                    continue;
                }
                if line == ":quit" || line == ":q" {
                    break;
                }
                if line == ":reset" {
                    runtime.reset();
                    println!("State cleared");
                    continue;
                }
                if line == ":env" {
                    let env = runtime.snapshot_env();
                    if env.is_empty() {
                        println!("(empty)");
                    } else {
                        for (_, row) in env {
                            println!("{}", row);
                        }
                    }
                    continue;
                }

                run_chunk(&mut runtime, line);
            }
            Err(err) => {
                eprintln!("Read error: {}", err);
                break;
            }
        }
    }
}

fn run_chunk(runtime: &mut Runtime, source: &str) {
    match eval_source(runtime, source) {
        Ok(result) => {
            for line in result.output {
                println!("{}", line);
            }
            println!("=> {}", result.last_value);
        }
        Err(err) => eprintln!("error: {}", err),
    }
}
