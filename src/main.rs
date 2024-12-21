use regex::Regex;

// yagrep usage:
// yagrep "hello world" file.txt
// yagrep "println" /path/to/directory
fn main() {
    let params = std::env::args().skip(1).collect::<Vec<String>>();
    if params.len() != 2 {
        println!("Usage: yagrep <pattern> <file>");
        return;
    }

    let pattern = params[0].clone();
    let file = params[1].clone();

    let re = match Regex::new(&pattern) {
        Ok(re) => re,
        Err(err) => {
            println!("Error compiling regex: {}", err);
            return;
        }
    };

    let path = get_full_path(&file);

    match (path.is_file(), path.is_dir()) {
        (true, false) => {
            match_file(&re, &path);
        }
        (false, true) => {
            match_directory(&re, &path);
        }
        (false, false) => {
            println!("Error: File not found");
        }
        _ => {}
    }
}

fn match_file(regex: &Regex, file: &std::path::Path) {
    let contents = match std::fs::read_to_string(file) {
        Ok(contents) => contents,
        Err(_err) => {
            return;
        }
    };

    for line in contents.lines() {
        if regex.is_match(line) {
            println!("{}", line);
        }
    }
}

fn match_directory(regex: &Regex, directory: &std::path::Path) {
    for entry in std::fs::read_dir(directory).expect("Failed to read directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.is_file() {
            match_file(regex, &path);
        } else if path.is_dir() {
            match_directory(regex, &path);
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
