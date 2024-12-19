use std::env;
use std::io;
use std::process;

fn match_pattern(input_line: &str, pattern: &str) -> bool {
    if pattern.chars().count() == 1 {
        input_line.contains(pattern)
    } else if pattern == "\\d" {
        match_single_digit(input_line)
    } else {
        panic!("Unhandled pattern: {}", pattern)
    }
}

fn match_single_digit(input_line: &str) -> bool {
    ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]
        .iter()
        .any(|&digit| input_line.contains(digit))
}

// Usage: echo <input_text> | your_program.sh -E <pattern>
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    //eprintln!("Logs from your program will appear here!");

    if env::args().nth(1).unwrap() != "-E" {
        println!("Expected first argument to be '-E'");
        process::exit(1);
    }

    let pattern = env::args().nth(2).unwrap();
    let mut input_line = String::new();

    io::stdin().read_line(&mut input_line).unwrap();

    if match_pattern(&input_line, &pattern) {
         process::exit(0)
    } else {
         process::exit(1)
    }
}
