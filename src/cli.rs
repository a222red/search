use std::sync::Arc;

use clap::{
    Command,
    Arg,
    crate_name,
    crate_authors,
    crate_version,
    crate_description,
    error::{ Error, ErrorKind }
};

pub struct Errors {
    pub bad_regex: Error,
    pub bad_path: Error,
    pub not_found: Error,
    pub io: Error,
    pub utf8: Error
}

pub fn gen_cli() -> (Command<'static>, Arc<Errors>) {
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

    return (
        cmd,
        errors
    );
}
