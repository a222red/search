mod fs;
mod search;

use crate::{
    fs::{get_files, read_to_string},
    search::{search, search_regex}
};

use std::{
    path::PathBuf,
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

pub struct Errors {
    bad_regex: Error,
    bad_path: Error,
    not_found: Error,
    io: Error,
    utf8: Error
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
        ),
        io: cmd.error(
            ErrorKind::ValueValidation,
            "IO error"
        ),
        utf8: cmd.error(
            ErrorKind::ValueValidation,
            "Invalid UTF-8 text"
        )
    });
    let m = cmd.get_matches_from(args_os());

    let search_str = m.value_of("SEARCH_STR")
        .unwrap_or_else(|| unreachable!());
    let pattern = match m.is_present("REGEX") {
        true => Pattern::RegEx(
            Regex::new(search_str)
                .unwrap_or_else(|_| errors.bad_regex.exit())
        ),
        false => Pattern::Str(Finder::new(search_str).into_owned())
    };

    match m.subcommand().unwrap_or_else(|| unreachable!()) {
        ("file", sm) => match pattern {
            Pattern::Str(s) => {
                let filename = sm.value_of("SEARCH_FILE").unwrap_or_else(
                    || unreachable!()
                );
                search(
                    filename,
                    &read_to_string(filename, errors.clone()),
                    &s,
                );
            },
            Pattern::RegEx(r) => {
                let filename = sm.value_of("SEARCH_FILE").unwrap_or_else(
                    || unreachable!()
                );
                search_regex(
                    filename,
                    &read_to_string(filename, errors.clone()),
                    &r,
                );
            }
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
                        file.as_os_str().to_str().unwrap_or_else(
                            || e.utf8.exit()
                        ),
                        &read_to_string(&file, e.clone()),
                        &c
                    )));
                }),
                Pattern::RegEx(r) => files.for_each(|file| {
                    let c = r.clone();
                    let e = errors.clone();
                    threads.push(spawn(move || search_regex(
                        file.as_os_str().to_str().unwrap_or_else(
                            || e.utf8.exit()
                        ),
                        &read_to_string(&file, e.clone()),
                        &c
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
                        file.as_os_str().to_str().unwrap_or_else(
                            || e.utf8.exit()
                        ),
                        &read_to_string(&file, e),
                        &c
                    )));
                }),
                Pattern::RegEx(r) => files.into_iter().for_each(|file| {
                    let c = r.clone();
                    let e = errors.clone();
                    threads.push(spawn(move || search_regex(
                        file.as_os_str().to_str().unwrap_or_else(
                            || e.utf8.exit()
                        ),
                        &read_to_string(&file, e),
                        &c
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
