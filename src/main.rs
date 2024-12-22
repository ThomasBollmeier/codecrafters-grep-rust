use std::env;
use std::io;
use std::process;

fn match_pattern(input_line: &str, pattern: &str) -> bool {
    if pattern.chars().count() == 1 {
        input_line.contains(pattern)
    } else if pattern == "\\d" {
        match_single_digit(input_line)
    } else if pattern == "\\w" {
        match_alphanumeric(input_line)
    } else if let Some('[') = pattern.chars().nth(0) {
        match_group(input_line, pattern)
    } else {
        panic!("Unhandled pattern: {}", pattern)
    }
}

fn match_group(input_line: &str, pattern: &str) -> bool {
    if pattern.chars().count() < 2 {
        return false;
    }

    let is_neg_group = pattern.chars().nth(1).unwrap() == '^';

    if !is_neg_group {
        let num_chars = pattern.chars().count() - 2;
        pattern
            .chars()
            .skip(1)
            .take(num_chars)
            .any(|ch| input_line.contains(ch))
    } else {
        let num_chars = input_line.chars().count() - 3;
        !pattern
            .chars()
            .skip(2)
            .take(num_chars)
            .any(|ch| input_line.contains(ch))
    }
}

fn match_alphanumeric(input_line: &str) -> bool {
    let lower_chars = "abcdefghijklmnopqrstuvwxyz";
    let upper_chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let digits = "0123456789";

    let mut alpha_nums = lower_chars.to_string();
    alpha_nums.push_str(&upper_chars);
    alpha_nums.push_str(&digits);
    alpha_nums.push('_');

    alpha_nums.chars().any(|ch| input_line.contains(ch))
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
