use crate::regex_parser::RegexParser;
use clap::Parser;
use std::{io, process};
use crate::matcher::Match;

mod matcher;
mod regex_parser;

#[derive(Debug, Clone)]
pub enum ColorMode {
    Always,
    Auto,
    Never,
}

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

    #[arg(long = "color", default_value = "never", value_parser = get_color_mode)]
    pub color: ColorMode,
}

fn get_color_mode(s: &str) -> Result<ColorMode, String> {
    match s {
        "always" => Ok(ColorMode::Always),
        "auto" => Ok(ColorMode::Auto),
        "never" => Ok(ColorMode::Never),
        _ => Err(format!(
            "'{s}' is not a valid value for --color. Use 'always', 'auto', or 'never'."
        )),
    }
}

pub fn process_stdin(config: &Config) {
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
        let matches = match_all(&line, &config.pattern);

        if matches.is_empty() {
            continue;
        }

        found = true;

        if !config.only_matches {
            println!("{}", colorize_line(&line, &matches, &config.color));
        } else {
            for m in matches {
                println!("{}", m.matched_text);
            }
        }
    }

    process::exit(if found { 0 } else { 1 });
}

pub fn process_files_or_dirs(config: &Config) {
    let mut found = false;
    let filenames: Vec<String> = if config.recursive {
        config.files_or_dirs
            .iter()
            .flat_map(|file_or_dir| get_files(file_or_dir))
            .collect()
    } else {
        config.files_or_dirs.to_vec()
    };

    let multiple_files = filenames.len() > 1;

    for filename in &filenames {
        let file_content = std::fs::read_to_string(filename).unwrap();

        for line in file_content.lines() {
            let matches = match_all(line, &config.pattern);
            if matches.is_empty() {
                continue;
            }
            found = true;

            if !config.only_matches {
                if multiple_files {
                    println!("{filename}:{}", colorize_line(line, &matches, &config.color));
                } else {
                    println!("{}", colorize_line(line, &matches, &config.color));
                }
            } else {
                for m in matches {
                    if multiple_files {
                        println!("{filename}:{}", m.matched_text);
                    } else {
                        println!("{}", m.matched_text);
                    }
                }
            }
        }
    }

    if found {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

fn colorize_line(line: &str, matches: &Vec<Match>, color_mode: &ColorMode) -> String {
    match color_mode {
        ColorMode::Always => {
            let mut colored_line = String::new();
            let mut last_index = 0;

            for m in matches {
                let start = m.offset;
                let end = start + m.matched_text.len();
                colored_line.push_str(&line[last_index..start]);
                colored_line.push_str("\x1b[1;31m"); // Start red color in bold
                colored_line.push_str(&line[start..end]);
                colored_line.push_str("\x1b[0m"); // Reset color
                last_index = end;
            }
            colored_line.push_str(&line[last_index..]);
            colored_line
        }
        _ => line.to_string(),
    }
}

fn match_all(input_line: &str, pattern: &str) -> Vec<Match> {
    RegexParser::new(pattern)
        .parse()
        .ok()
        .map_or(vec![], |m| m.find_all_matches(input_line))
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
