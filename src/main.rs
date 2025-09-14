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

#[derive(Deserialize, Serialize)]
struct Book {
    author: Option<String>,
    title: String,
    year: Option<u16>,
    cover: String,
    location: Option<String>,
    condition: u8,
    edition: Option<String>,
    publisher: Option<String>,
    category: u16,
    description: String,
    language: String,
    isbn: Option<String>,
    pages: String,
    format: String,
    weight: u16,
    price: u16,
    cover_url: Option<String>,
    keywords: Option<Vec<String>>,
    new: bool,
    first_edition: bool,
    signed: bool,
    unused: bool,
    personal_notice: Option<String>,
    unlimited: bool,
}

impl Book {
    fn new(
        author: Option<String>,
        title: String,
        year: Option<u16>,
        cover: String,
        location: Option<String>,
        condition: u8,
        edition: Option<String>,
        publisher: Option<String>,
        category: u16,
        description: String,
        language: String,
        isbn: Option<String>,
        pages: String,
        format: String,
        weight: u16,
        price: u16,
        cover_url: Option<String>,
        keywords: Option<Vec<String>>,
        new: bool,
        first_edition: bool,
        signed: bool,
        unused: bool,
        personal_notice: Option<String>,
        unlimited: bool,
    ) -> Self {
        Self {
            author,
            title,
            year,
            cover,
            location,
            condition,
            edition,
            publisher,
            category,
            description,
            language,
            isbn,
            pages,
            format,
            weight,
            price,
            cover_url,
            keywords,
            new,
            first_edition,
            signed,
            unused,
            personal_notice,
            unlimited,
        }
    }
}

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

    let filenameregex = Regex::new(r"^\d{5}.toml$").unwrap();

    files.retain(|f| filenameregex.is_match(&f.as_bytes()));

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

    let mut deserdata = HashMap::new();

    for mut file in bookfiles {
        let mut content = String::new();
        file.0.read_to_string(&mut content).expect("");
        let deserialized: Result<Table, Error> = toml::from_str(&content.as_str());
        match deserialized {
            Ok(_) => deserdata.insert(file.1[..5].to_string(), (deserialized.unwrap(), file)),
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

    let mut emptyfiles = Vec::new();

    for (id, content) in &deserdata {
        if content.0 == Table::default() {
            content.1 .0.unlock().unwrap();
            remove_file(datapath.join(content.1 .1.clone())).unwrap();
            emptyfiles.push(id.clone());
        }
    }

    for file in emptyfiles {
        let _ = &deserdata.remove(&file);
    }

    let book_entries: Vec<booklistentry> = deserdata
        .keys()
        .into_iter()
        .map(|key| booklistentry {
            id: key.clone().into(),
        })
        .collect();

    let books_model = std::rc::Rc::new(slint::VecModel::from(book_entries));
    let main_window = MainWindow::new()?;

    main_window.set_entries(books_model.clone().into());

    main_window.run()
}
