use std::env;
use std::io;
use std::process;
use codecrafters_grep::regex_parser::RegexParser;

fn process_stdin(pattern: &str) {
    let mut input_line = String::new();
    io::stdin().read_line(&mut input_line).unwrap();

    if match_pattern(&input_line, pattern) {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

fn process_file(filename: &str, pattern: &str) {
    let file_content = std::fs::read_to_string(filename).unwrap();
    let mut found = false;

    for line in file_content.lines() {
        if match_pattern(line, pattern) {
            found = true;
            println!("{line}");
        }
    }

    if found {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

fn process_files(filenames: &[String], pattern: &str) {
    let mut found = false;

    for filename in filenames {
        let file_content = std::fs::read_to_string(filename).unwrap();

        for line in file_content.lines() {
            if match_pattern(line, pattern) {
                found = true;
                println!("{filename}:{line}");
            }
        }
    }

    if found {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

fn match_pattern(input_line: &str, pattern: &str) -> bool {
    match RegexParser::new(pattern).parse() {
        Ok(matcher) => matcher.matches(&input_line),
        Err(_) => false,
    }
}

// Usage: echo <input_text> | your_program.sh -E <pattern>
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    //eprintln!("Logs from your program will appear here!");

    let num_args = env::args().len();
    if num_args < 3 {
        println!("Expected 2 or more arguments, got {}", num_args - 1);
        process::exit(1);
    }

    if env::args().nth(1).unwrap() != "-E" {
        println!("Expected first argument to be '-E'");
        process::exit(1);
    }

    let pattern = env::args().nth(2).unwrap();

    match num_args {
        3 => process_stdin(&pattern),
        4 => process_file(&env::args().nth(3).unwrap(), &pattern),
        _ => {
            let filenames: Vec<String> = env::args().skip(3).collect();
            process_files(&filenames, &pattern)
        }
    }
}
