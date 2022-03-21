use std::{
    fs::{read_dir, read_to_string},
    path::{Path, PathBuf},
    str::FromStr,
    thread::{spawn, JoinHandle},
    sync::Arc
};

use clap::{
    Command,
    Arg,
    crate_name,
    crate_authors,
    crate_version,
    crate_description,
    error::{Error, ErrorKind}
};

use wild::args_os;

use memchr::memmem::Finder;

use regex::Regex;

enum Pattern<'a> {
    Str(Finder<'a>),
    RegEx(Regex)
}

struct Errors {
    bad_regex: Error,
    bad_path: Error,
    not_found: Error
}

fn get_files<P: AsRef<Path>>(
    path: P,
    exclude_dirs: &Vec<PathBuf>,
    exclude_files: &Vec<PathBuf>,
    errors: Arc<Errors>
) -> Vec<PathBuf> {
    let mut out = Vec::<PathBuf>::new();
    
    for opt in read_dir(path).unwrap_or_else(
        |_| errors.not_found.exit()
    ) {
        let entry = opt.unwrap();
        let mut exclude = false;
        for dir in exclude_dirs {
            if entry.path().starts_with(dir) {
                exclude = true;
                break;
            }
        }
        for file in exclude_files {
            if entry.path() == file.as_path() {
                exclude = true;
                break;
            }
        }
        if !exclude {
            match entry.file_type().unwrap().is_dir() {
                true => out.append(
                    &mut get_files(
                        entry.path(),
                        exclude_dirs,
                        exclude_files,
                        errors.clone()
                    )
                ),
                false => out.push(entry.path())
            }
        }
    }

    return out;
}

fn search<P: AsRef<Path>>(file: P, pattern: &Finder, errors: Arc<Errors>) {
    let mut num = 0usize;
    
    read_to_string(&file).unwrap_or_else(
        |_| errors.not_found.exit()
    ).lines().for_each(|line| {
        num += 1;
        if let Some(i) = pattern.find(line.as_bytes()) {
            println!(
                "\u{001b}[34;1m[{}:{}]:\u{001b}[0m {}",
                file.as_ref().to_str().unwrap(),
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

fn search_regex<P: AsRef<Path>>(file: P, pattern: &Regex, errors: Arc<Errors>) {
    let mut num = 0usize;
    read_to_string(&file).unwrap_or_else(
        |_| errors.not_found.exit()
    ).lines().for_each(|line| {
        num += 1;
        if let Some(mat) = pattern.find(line) {
            println!(
                "\u{001b}[33m[{}:{}]:\u{001b}[0m {}",
                file.as_ref().to_str().unwrap(),
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

fn main() {
    let mut cmd = Command::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .subcommand_required(true)
        .arg(Arg::new("SEARCH_STR")
            .help("String to search for")
            .index(1)
            .required(true)
        )
        .arg(Arg::new("REGEX")
            .help("Treat search string as a regular expression")
            .long("regex")
            .short('r')
        )
        .subcommand(Command::new("file")
            .visible_alias("f")
            .arg(Arg::new("SEARCH_FILE")
                .help("File to search in")
                .index(1)
                .required(true)
            )
        )
        .subcommand(Command::new("files")
            .visible_alias("s")
            .arg(Arg::new("SEARCH_FILES")
                .help("Files to search in")
                .index(1)
                .multiple_values(true)
                .required(true)
            )
        )
        .subcommand(Command::new("directory")
            .visible_alias("dir")
            .visible_alias("d")
            .arg(Arg::new("SEARCH_DIR")
                .help("Directory to search in")
                .index(1)
                .default_value(".")
            )
            .arg(Arg::new("EXCLUDE_DIRS")
                .help("List of directories not to search in")
                .long("exclude-dirs")
                .takes_value(true)
                .multiple_values(true)
                .value_name("DIRS")
            )
            .arg(Arg::new("EXCLUDE_FILES")
                .help("List of files not to search in")
                .long("exclude-files")
                .takes_value(true)
                .multiple_values(true)
                .value_name("FILES")
            )
        );
    let errors = Arc::new(Errors {
        bad_regex: cmd.error(
            ErrorKind::ValueValidation,
            "Invalid Regex pattern"
        ),
        bad_path: cmd.error(
            ErrorKind::ValueValidation,
            "Invalid path"
        ),
        not_found: cmd.error(
            ErrorKind::ValueValidation,
            "File or directory not found"
        )
    });
    let m = cmd.get_matches_from(args_os());

    let string = m.value_of("SEARCH_STR")
        .unwrap_or_else(|| unreachable!())
        .to_owned();
    let pattern = match m.is_present("REGEX") {
        true => Pattern::RegEx(
            Regex::new(string.as_str())
                .unwrap_or_else(|_| errors.bad_regex.exit())
        ),
        false => Pattern::Str(Finder::new(string.as_str()).into_owned())
    };

    match m.subcommand().unwrap_or_else(|| unreachable!()) {
        ("file", sm) => match pattern {
            Pattern::Str(s) => search(
                sm.value_of("SEARCH_FILE").unwrap_or_else(
                    || unreachable!()
                ),
                &s,
                errors.clone()
            ),
            Pattern::RegEx(r) => search_regex(
                sm.value_of("SEARCH_FILE").unwrap_or_else(
                    || unreachable!()
                ),
                &r,
                errors.clone()
            )
        },
        ("files", sm) => {
            let files = sm.values_of("SEARCH_FILES")
                .unwrap_or_else(|| unreachable!())
                .map(|s| PathBuf::from_str(s).unwrap_or_else(
                    |_| errors.bad_path.exit()
                ));
            let mut threads = Vec::<JoinHandle<()>>::new();
            match pattern {
                Pattern::Str(s) => files.for_each(|file| {
                    let c = s.clone();
                    let e = errors.clone();
                    threads.push(spawn(move || search(
                        file,
                        &c,
                        e
                    )));
                }),
                Pattern::RegEx(r) => files.for_each(|file| {
                    let c = r.clone();
                    let e = errors.clone();
                    threads.push(spawn(move || search_regex(
                        file,
                        &c,
                        e
                    )));
                })
            }
        },
        ("directory", sm) => {
            let exclude_dirs = match sm.values_of("EXCLUDE_DIRS") {
                Some(i) => i.map(
                    |s| PathBuf::from_str(s).unwrap_or_else(
                        |_| errors.bad_path.exit()
                    )
                ).collect(),
                None => vec![]
            };
            let exclude_files = match sm.values_of("EXCLUDE_FILES") {
                Some(i) => i.map(
                    |s| PathBuf::from_str(s).unwrap_or_else(
                        |_| errors.bad_path.exit()
                    )
                ).collect(),
                None => vec![]
            };
            let files = get_files(
                sm.value_of("SEARCH_DIR").unwrap_or_else(
                    || unreachable!()
                ),
                &exclude_dirs,
                &exclude_files,
                errors.clone()
            );
            let mut threads = Vec::<JoinHandle<()>>::new();

            match pattern {
                Pattern::Str(f) => files.into_iter().for_each(|file| {
                    let c = f.clone();
                    let e = errors.clone();
                    threads.push(spawn(move || search(
                        file,
                        &c,
                        e
                    )));
                }),
                Pattern::RegEx(r) => files.into_iter().for_each(|file| {
                    let c = r.clone();
                    let e = errors.clone();
                    threads.push(spawn(move || search_regex(
                        file,
                        &c,
                        e
                    )));
                })
            }
        
            threads.into_iter().for_each(|thread| {
                thread.join().unwrap_or_else(
                    |_| unreachable!()
                );
            });
        },
        _ => unreachable!()
    }
}
