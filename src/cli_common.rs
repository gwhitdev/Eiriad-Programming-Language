use std::env;
use std::fs;
use std::io::{self, Read, Write};

use crate::{eval_source, Runtime};

pub fn run_cli(allow_inline_and_stdin: bool) {
    let mut runtime = Runtime::new();
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        if allow_inline_and_stdin {
            match args[1].as_str() {
                "-c" | "--code" => {
                    if args.len() < 3 {
                        eprintln!("error: {} requires a code argument", args[1]);
                        std::process::exit(2);
                    }
                    run_chunk(&mut runtime, &args[2], true);
                    return;
                }
                "-" => {
                    let mut source = String::new();
                    if io::stdin().read_to_string(&mut source).is_err() {
                        eprintln!("error: failed to read from stdin");
                        std::process::exit(1);
                    }
                    run_chunk(&mut runtime, &source, true);
                    return;
                }
                _ => {}
            }
        }

        let path = &args[1];
        match fs::read_to_string(path) {
            Ok(source) => run_chunk(&mut runtime, &source, true),
            Err(err) => {
                eprintln!("Failed to read {}: {}", path, err);
                std::process::exit(1);
            }
        }
        return;
    }

    start_repl(&mut runtime);
}

fn start_repl(runtime: &mut Runtime) {
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

                run_chunk(runtime, line, false);
            }
            Err(err) => {
                eprintln!("Read error: {}", err);
                break;
            }
        }
    }
}

fn run_chunk(runtime: &mut Runtime, source: &str, hard_fail: bool) {
    match eval_source(runtime, source) {
        Ok(result) => {
            for line in result.output {
                println!("{}", line);
            }
            println!("=> {}", result.last_value);
        }
        Err(err) => {
            eprintln!("error: {}", err);
            if hard_fail {
                std::process::exit(1);
            }
        }
    }
}
