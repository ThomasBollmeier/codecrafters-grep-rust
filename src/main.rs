use std::env;
use std::io;
use std::process;
use codecrafters_grep::matcher::*;

fn match_pattern(input_line: &str, pattern: &str) -> bool {
    let matcher: Box<dyn Matcher> = if pattern.chars().count() == 1 {
        Box::new(SingleCharMatcher::new(pattern.chars().next().unwrap()))
    } else if pattern == "\\d" {
        Box::new(make_digit_matcher())
    } else if pattern == "\\w" {
        Box::new(make_alpha_num_matcher())
    } else if let Some('[') = pattern.chars().nth(0) {
        Box::new(make_group_matcher(pattern))
    } else {
        Box::new(SequenceMatcher::from_pattern(pattern).expect("invalid pattern"))
    };

    matcher.matches(input_line)
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
