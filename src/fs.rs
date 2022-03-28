use crate::cli::Errors;

use std::{
    fs::{read_dir, File},
    path::{Path, PathBuf},
    sync::Arc,
    io::Read
};

pub fn get_files<P: AsRef<Path>>(
    path: P,
    exclude_dirs: &Vec<PathBuf>,
    exclude_files: &Vec<PathBuf>,
    errors: Arc<Errors>
) -> Vec<PathBuf> {
    let mut out = Vec::<PathBuf>::new();
    
    for opt in read_dir(path).unwrap_or_else(
        |_| errors.not_found.exit()
    ) {
        let entry = opt.unwrap_or_else(
           |_| errors.io.exit()
        );
        let mut exclude = false;
        loop {
            for dir in exclude_dirs {
                if entry.path().starts_with(dir) {
                    exclude = true;
                    break;
                }
            }
            if exclude {break}
            for file in exclude_files {
                if entry.path() == file.as_path() {
                    exclude = true;
                    break;
                }
            }
            break;
        }
        if !exclude {
            match entry.file_type().unwrap_or_else(
                |_| unreachable!()
            ).is_dir() {
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

pub fn read_to_string<P: AsRef<Path> + ?Sized>(file: &P, errors: Arc<Errors>) -> String {
    let mut buf = String::new();

    File::open(file).unwrap_or_else(
        |_| errors.not_found.exit()
    ).read_to_string(&mut buf).unwrap_or_else(
        |_| errors.utf8.exit()
    );

    return buf;
}
