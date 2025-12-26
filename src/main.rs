use clap::Parser;
use codecrafters_grep::{process_files_or_dirs, process_stdin, Config};
use std::process;

fn main() {
    let config = Config::parse();

    if !config.extended_regexp {
        println!("Option '-E' must be set to use extended regular expressions.");
        process::exit(1);
    }

    match config.files_or_dirs.len() {
        0 => process_stdin(&config.pattern, config.only_matches),
        _ => process_files_or_dirs(
            &config.files_or_dirs,
            &config.pattern,
            config.recursive,
            config.only_matches,
        ),
    }
}
