use std::io::{BufWriter, Write};

use colored::*;
use regex::{Regex, RegexBuilder};

const USAGE: &'static str = "Usage: yagrep [options] <pattern> <file>";

#[derive(PartialEq)]
enum CliOptions {
    IgnoreCase,
    IgnoreGitIgnore,
    IgnoreNoHiddenFiles,
    Empty,
}

struct CliApp {
    options: Vec<CliOptions>,
    pattern: String,
    path: std::path::PathBuf,
    ignored_paths: std::cell::RefCell<Vec<std::path::PathBuf>>,
}

impl CliApp {
    fn new(args: Vec<String>) -> Result<CliApp, &'static str> {
        if args.len() < 3 {
            return Err(USAGE);
        }

        let pattern = args[1].clone();
        let path = get_full_path(&args[2]);

        let options = args
            .iter()
            .filter(|&arg| arg.starts_with("-"))
            .map(|arg| {
                arg.as_str().chars().map(|c| match c {
                    'i' => CliOptions::IgnoreCase,
                    'g' => CliOptions::IgnoreGitIgnore,
                    'H' => CliOptions::IgnoreNoHiddenFiles,
                    _ => CliOptions::Empty,
                })
            })
            .flatten()
            .collect();

        Ok(CliApp {
            options,
            pattern,
            path,
            ignored_paths: std::cell::RefCell::new(Vec::new()),
        })
    }

    fn has_option(&self, option: CliOptions) -> bool {
        self.options.contains(&option)
    }
}

fn is_git_ignore(git_dir_path: &std::path::Path, path: &std::path::Path) -> Option<bool> {
    let output = match std::process::Command::new("git")
        .arg("-C")
        .arg(git_dir_path)
        .arg("check-ignore")
        .arg(path)
        .output()
    {
        Ok(output) => output,
        Err(_) => return None,
    };
    output.status.success().then(|| Some(true))?
}

fn git_root(path: &std::path::Path) -> Option<std::path::PathBuf> {
    let output = match std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
    {
        Ok(output) => output,
        Err(_) => return None,
    };
    output.status.success().then(|| {
        std::str::from_utf8(&output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
            .map(std::path::PathBuf::from)
    })?
}

fn main() {
    let params = std::env::args().collect::<Vec<String>>();
    let app = match CliApp::new(params) {
        Ok(app) => app,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let pattern = &app.pattern;
    let path = &app.path;

    let mut regex_builder_binding = RegexBuilder::new(pattern);
    let regex_builder =
        regex_builder_binding.case_insensitive(app.has_option(CliOptions::IgnoreCase));

    let re = match regex_builder.build() {
        Ok(re) => re,
        Err(err) => {
            println!("Error compiling regex: {}", err);
            return;
        }
    };

    match (path.is_file(), path.is_dir()) {
        (true, false) => {
            match_file(&re, &path);
        }
        (false, true) => {
            match_directory(&re, &path, &app);
        }
        (false, false) => {
            println!("Error: File not found");
        }
        _ => {}
    }
}

fn match_file(regex: &Regex, path: &std::path::Path) {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_err) => {
            return;
        }
    };

    let mut writer = BufWriter::new(std::io::stdout());
    // let mut writer = std::io::LineWriter::new(writer);
    let mut matches = contents
        .lines()
        .enumerate()
        .filter(|(_index, line)| regex.is_match(line))
        .peekable();
    if matches.peek().is_some() {
        write!(writer, "{}\n", path.display().to_string().green()).unwrap();
    }
    for (index, line) in matches {
        write!(writer, "{}: {}\n", index + 1, line).unwrap();
    }
}

fn match_directory(regex: &Regex, directory: &std::path::Path, app: &CliApp) {
    for entry in std::fs::read_dir(directory).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if !app.has_option(CliOptions::IgnoreNoHiddenFiles)
            && path.file_name().unwrap().to_str().unwrap().starts_with(".")
        {
            continue;
        }
        if !app.has_option(CliOptions::IgnoreGitIgnore) {
            if app
                .ignored_paths
                .borrow()
                .iter()
                .any(|p| p.starts_with(&path))
            {
                continue;
            }
            let git_root = git_root(&path);
            if let Some(git_root) = &git_root {
                if is_git_ignore(git_root, &path) == Some(true) {
                    app.ignored_paths
                        .borrow_mut()
                        .push(std::path::PathBuf::from(&path));
                    continue;
                }
            }
        }
        if path.is_file() {
            match_file(regex, &path);
        } else if path.is_dir() {
            match_directory(regex, &path, app);
        }
    }
}

fn get_full_path(path: &str) -> std::path::PathBuf {
    match path
        .chars()
        .next()
        .expect("Failed to get first character of path")
    {
        '/' => std::path::PathBuf::new().join(path),
        _ => {
            let current_dir = std::env::current_dir().expect("Failed to get current directory");
            current_dir.join(path)
        }
    }
}
