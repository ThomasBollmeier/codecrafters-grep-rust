use std::env;
use std::io;
use std::process;
use codecrafters_grep::regex_parser::RegexParser;

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
