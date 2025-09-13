use nix::fcntl::{Flock, FlockArg};
use regex::bytes::Regex;
use std::fs::OpenOptions;
use std::fs::{self, create_dir_all, File};
use std::io;
use std::io::prelude::*;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use toml::Table;
use walkdir::{DirEntry, WalkDir};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let datapath = Path::new("/home/robertd/test/Antiquar");
    if datapath.try_exists().is_err() {
        create_dir_all(datapath).map_err(|e| {
            eprintln!("Failed to create data directory: {}", e);
            slint::PlatformError::Other(format!("Directory creation failed: {}", e))
        })?;
    }
    // scan for files
    let mut files: Vec<String> = vec![];

    for entry in WalkDir::new(datapath)
        .max_depth(1)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|e| {
            e.metadata().unwrap().is_file()
                && e.path().extension().map_or(false, |ext| ext == "toml")
        })
    {
        files.push(entry.file_name().to_str().unwrap().to_string())
    }
    println!("{:#?}", files);

    let filenameregex = Regex::new(r"^\d{5}.toml$").unwrap();

    files.retain(|f| filenameregex.is_match(&f.as_bytes()));

    println!("{:#?}", files);

    let mut bookfiles = vec![];

    for f in files {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(datapath.join(f))
            .unwrap();
        let lock = Flock::lock(file, FlockArg::LockExclusive).expect("Couldn't acquire lock");

        bookfiles.push(lock);
    }

    let main_window = MainWindow::new()?;

    main_window.run()
}
