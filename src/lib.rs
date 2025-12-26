use crate::regex_parser::RegexParser;
use clap::Parser;
use std::{io, process};
use crate::matcher::Match;

mod matcher;
mod regex_parser;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Config {
    pub pattern: String,

    #[arg()]
    pub files_or_dirs: Vec<String>,

    #[arg(short = 'E', long)]
    pub extended_regexp: bool,

    #[arg(short, long)]
    pub recursive: bool,

    #[arg(short, long)]
    pub only_matches: bool,
}

pub fn process_stdin(pattern: &str, only_matches: bool) {
    let mut input_lines = vec![];

    loop {
        let mut buffer = String::new();
        let bytes_read = io::stdin().read_line(&mut buffer).unwrap();
        if bytes_read == 0 {
            break; // EOF reached
        }
        input_lines.push(buffer);
    }

    let mut found = false;

    for input_line in input_lines {
        let line = input_line.trim_end_matches(&['\n', '\r'][..]);
        match match_pattern(&line, pattern) {
            Some(matched) => {
                found = true;
                if !only_matches {
                    println!("{line}");
                } else {
                    println!("{}", matched.matched_text);
                }
            }
            None => {}
        }
    }

    process::exit(if found { 0 } else { 1 });
}

pub fn process_files_or_dirs(
    file_or_dirs: &[String],
    pattern: &str,
    recursive: bool,
    only_matches: bool,
) {
    let mut found = false;
    let filenames: Vec<String> = if recursive {
        file_or_dirs
            .iter()
            .flat_map(|file_or_dir| get_files(file_or_dir))
            .collect()
    } else {
        file_or_dirs.to_vec()
    };

    let multiple_files = filenames.len() > 1;

    for filename in &filenames {
        let file_content = std::fs::read_to_string(filename).unwrap();

        for line in file_content.lines() {
            match match_pattern(line, pattern) {
                Some(matched) => {
                    found = true;
                    let s = if only_matches {
                        matched.matched_text
                    } else {
                        line.to_string()
                    };
                    if multiple_files {
                        println!("{filename}:{s}");
                    } else {
                        println!("{s}");
                    }
                }
                None => {}
            }
        }
    }

    if found {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

fn match_pattern(input_line: &str, pattern: &str) -> Option<Match> {
    match RegexParser::new(pattern).parse() {
        Ok(matcher) => matcher.find_match(&input_line),
        Err(_) => None,
    }
}

fn get_files(file_or_dir: &str) -> Vec<String> {
    let path = std::path::Path::new(file_or_dir);
    if path.is_dir() {
        match get_files_in_directory(path) {
            Ok(files) => files
                .into_iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            Err(_) => vec![],
        }
    } else if path.is_file() {
        vec![file_or_dir.to_string()]
    } else {
        vec![]
    }
}

fn get_files_in_directory(dir: &std::path::Path) -> io::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            files.extend(get_files_in_directory(&entry.path())?);
        } else {
            files.push(entry.path());
        }
    }
    Ok(files)
}
