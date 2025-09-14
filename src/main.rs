use fs4::fs_std::FileExt;
use regex::bytes::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::fs::remove_file;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;
use toml::de::Error;
use toml::Table;
use walkdir::WalkDir;

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
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(datapath.join(f.clone()))
            .unwrap();
        if file.try_lock_exclusive().unwrap() == false {
            eprintln!("Failed to acquire lock for file: {:#?}", datapath.join(f));
            panic!()
        }

        bookfiles.push((file, f));
    }

    println!("{:#?}", bookfiles);

    let mut deserdata = HashMap::new();

    for mut file in bookfiles {
        let mut content = String::new();
        file.0.read_to_string(&mut content).expect("");
        let dlized: Result<Table, Error> = toml::from_str(&content.as_str());
        match dlized {
            Ok(_) => deserdata.insert(file.1[..5].to_string(), (dlized.unwrap(), file)),
            Err(_) => {
                eprintln!(
                    "Could'nt parse toml file {:#?} with content:\n\n{}",
                    datapath.join(file.1),
                    content
                );
                panic!()
            }
        };
    }

    println!("{:#?}", deserdata);

    let mut emptyfiles = Vec::new();

    for (id, content) in &deserdata {
        println!("{id}");
        println!("{content:#?}");
        if content.0 == Table::default() {
            content.1 .0.unlock().unwrap();
            remove_file(datapath.join(content.1 .1.clone())).unwrap();
            emptyfiles.push(id.clone());
        }
    }

    for file in emptyfiles {
        &deserdata.remove(&file);
    }

    let main_window = MainWindow::new()?;

    main_window.run()
}
