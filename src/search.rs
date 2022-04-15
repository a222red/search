use memchr::memmem::Finder;

use regex::Regex;

pub fn search(filename: &str, text: &String, pattern: &Finder) {
    let mut num = 0usize;

    text.lines().for_each(|line| {
        num += 1;

        if let Some(i) = pattern.find(line.as_bytes()) {
            println!(
                "\u{001b}[34;1m[{}:{}]:\u{001b}[0m {}",
                filename,
                num,
                format!(
                    "{}\u{001b}[32m{}\u{001b}[0m{}",
                    &line[..i],
                    &line[i..(i + pattern.needle().len())],
                    &line[(i + pattern.needle().len())..]
                ).trim()
            );
        }
    });
}

pub fn search_regex(filename: &str, text: &String, pattern: &Regex) {
    let mut num = 0usize;

    text.lines().for_each(|line| {
        num += 1;
        if let Some(mat) = pattern.find(line) {
            println!(
                "\u{001b}[34m[{}:{}]:\u{001b}[0m {}",
                filename,
                num,
                format!(
                    "{}\u{001b}[32m{}\u{001b}[0m{}",
                    &line[..(mat.start())],
                    &line[(mat.start())..(mat.end())],
                    &line[(mat.end())..]
                ).trim()
            );
        }
    });
}
