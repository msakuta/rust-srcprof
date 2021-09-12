use std::{ffi::OsStr, io::BufRead, ops::Not};
use regex::Regex;

use lazy_static::lazy_static;

pub struct IgnoreDirs {
}

impl IgnoreDirs {
    pub fn query_from(root_dir: &str) -> Vec<String> {
        let ignore_files_path = IgnoreDirs::get_all_ignore_files(root_dir);
        
        ignore_files_path.iter()
            .map(|f| IgnoreDirs::get_all_ignore_paths_from_ignore_file(f))
            .flatten()
            .collect::<Vec<String>>()
    }

    fn is_ignore_file_pattern(input: &OsStr) -> bool {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^.[a-z]*ignore$")
            .expect("failed to create regex");
        }

        RE.is_match(input.to_str().unwrap())
    }

    fn get_all_ignore_files(dir: &str) -> Vec<String> {
        std::fs::read_dir(dir)
            .unwrap()
            .into_iter()
            .map(|file| file.unwrap().file_name())
            .filter(|file_name| IgnoreDirs::is_ignore_file_pattern(&file_name))
            .map(|f| format!("{}/{}", dir, f.to_str().unwrap()))
            .collect::<Vec<String>>()
    }

    fn is_ignore_path_pattern(input: &str) -> bool {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(^#.*|^\s*$)")
            .expect("failed to create regex");
        }

        RE.is_match(input).not()
    }

    fn get_all_ignore_paths_from_ignore_file(input: &str) -> Vec<String> {        
        let file = std::fs::File::open(input)
            .expect(format!("failed to open file ({})", input).as_str());
        
        std::io::BufReader::new(file)
            .lines()
            .map(|line| line.unwrap())    
            .filter(|line| IgnoreDirs::is_ignore_path_pattern(line))
            .collect::<Vec<String>>()
    }
}

#[cfg(test)]
mod tests {
    use crate::ignore_dirs::IgnoreDirs;

    #[test]
    fn check_is_ignore_file_pattern() {
        let samples = [
            (".gitignore", true),
            (".dockerignore", true),
            (".ignore", true),
            ("src", false),
            ("build", false),
            (".123ignore", false),
            ("README.md", false),
            (".IGNORE", false)
        ];
        
        for (input, expected) in samples {
            assert_eq!(
                IgnoreDirs::is_ignore_file_pattern(std::ffi::OsStr::new(input)), 
                expected, 
                "input ({}) should be {}", input, expected);
        }
    }

    #[test]
    fn check_is_ignore_path_pattern() {
        let samples = [
            ("# build directory", false),
            ("", false),
            ("/build", true),
            ("build", true),
            (".editorconfig", true)
        ];
        
        for (input, expected) in samples {
            assert_eq!(
                IgnoreDirs::is_ignore_path_pattern(input), 
                expected, 
                "input ({}) should be {}", input, expected);
        }
    }
}