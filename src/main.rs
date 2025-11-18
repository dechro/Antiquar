use fs4::fs_std::FileExt;
use gpui::http_client::anyhow;
use gpui_component::input::Input;
use gpui_component::input::InputEvent;
use gpui_component::input::InputState;
use gpui_component::tag::Tag;
use regex::bytes::Regex;
use serde::{Deserialize, Serialize};
use std::alloc::Layout;
use std::borrow::Cow;
use std::env;
use std::fs::create_dir_all;
use std::fs::remove_file;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use toml::de::Error;
use toml::Table;
use walkdir::WalkDir;

use rust_embed::RustEmbed;

use gpui::*;
use gpui_component::{button::*, *};
use gpui_component::{
    h_virtual_list,
    scroll::{Scrollbar, ScrollbarAxis, ScrollbarState},
    v_virtual_list, Icon, IconName, VirtualListScrollHandle,
};

use gpui::{px, size, Pixels, ScrollStrategy, Size};
use gpui_component::{ActiveTheme, Theme};
use std::rc::Rc;

#[derive(RustEmbed)]
#[folder = "./assets"]
#[include = "icons/**/*.svg"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Bookdata {
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

impl Default for Bookdata {
    fn default() -> Self {
        Self {
            author: None,
            title: String::new(),
            year: None,
            cover: String::new(),
            location: None,
            condition: 0,
            edition: None,
            publisher: None,
            category: 0,
            description: String::new(),
            language: String::new(),
            isbn: None,
            pages: String::new(),
            format: String::new(),
            weight: 0,
            price: 0,
            cover_url: None,
            keywords: None,
            new: false,
            first_edition: false,
            signed: false,
            unused: false,
            personal_notice: None,
            unlimited: false,
        }
    }
}

pub struct MainWindow {
    items: Vec<(u32, Option<Bookdata>, Arc<File>)>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    scroll_handle: VirtualListScrollHandle,
    search_input: Entity<InputState>,
    search_input_value: SharedString,
    _subscriptions: Vec<Subscription>,
    scroll_state: ScrollbarState,
}

impl MainWindow {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        book_list: Vec<(u32, Option<Bookdata>, Arc<File>)>,
    ) -> Self {
        let item_sizes = Rc::new(
            book_list
                .iter()
                .map(|_| size(px(200.), px(3000.)))
                .collect(),
        );
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Suchen..."));
        let _subscriptions = vec![cx.subscribe_in(&search_input, window, {
            let search_input = search_input.clone();
            move |this, _, ev: &InputEvent, _window, cx| match ev {
                InputEvent::Change => {
                    let value = search_input.read(cx).value();
                    this.search_input_value = value.into();
                    cx.notify()
                }
                _ => {}
            }
        })];
        Self {
            items: book_list,
            item_sizes,
            scroll_handle: VirtualListScrollHandle::new(),
            search_input,
            search_input_value: SharedString::default(),
            _subscriptions,
            scroll_state: ScrollbarState::default(),
        }
    }
}

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items = Rc::new(self.items.clone());
        let item_sizes = Rc::new(self.item_sizes.clone());
        let scroll_handle = self.scroll_handle.clone();
        let mut ui = div()
            .size_full()
            .p_2()
            .gap_1()
            .child(
                div()
                    .p_2()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("create_new_button")
                            .icon(Icon::new(Icon::empty()).path("icons/circle-plus.svg"))
                            .label("Neu")
                            .primary(),
                    )
                    .child(
                        Input::new(&self.search_input)
                            .suffix(
                                Button::new("filter_search")
                                    .ghost()
                                    .icon(Icon::new(Icon::empty()).path("icons/funnel.svg"))
                                    .xsmall(),
                            )
                            .max_w(px(300.0))
                            .min_w(px(150.)),
                    )
                    .child(
                        Button::new("status_indicator")
                            .ghost()
                            .child(
                                Icon::new(Icon::empty())
                                    .path("icons/circle.svg")
                                    .small()
                                    .text_color(cx.theme().red),
                            )
                            .child("text")
                            .child(
                                Icon::new(Icon::empty())
                                    .path("icons/circle.svg")
                                    .small()
                                    .text_color(cx.theme().blue),
                            )
                            .child("texttest"),
                    )
                    .child(div().flex_grow())
                    .child(
                        Button::new("menu")
                            .ghost()
                            .icon(Icon::new(Icon::empty()).path("icons/menu.svg")),
                    ),
            )
            .child(
                div()
                    .relative()
                    .size_full()
                    .child(
                        v_virtual_list(
                            cx.entity().clone(),
                            "main_booklist",
                            self.item_sizes.clone(),
                            move |view, visible_range, _, cx| {
                                visible_range
                                    .map(|ix| {
                                        div()
                                            .h(view.item_sizes[ix].height)
                                            .w_full()
                                            .bg(cx.theme().secondary)
                                            .child(format!("{:#?}", items[ix]))
                                    })
                                    .collect()
                            },
                        )
                        .track_scroll(&self.scroll_handle)
                        .p_2()
                        .border_1(),
                    )
                    .child(
                        // Add scrollbars
                        div()
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .bottom_0()
                            .child(
                                Scrollbar::both(&self.scroll_state, &self.scroll_handle)
                                    .axis(ScrollbarAxis::Vertical),
                            ),
                    ),
            );
        ui
    }
}

fn main() {
    let config = load_config();
    let books = load_data(Path::new(&config.datapath));

    // let books_model = std::rc::Rc::new(slint::VecModel::from(books));
    let app = Application::new().with_assets(Assets);
    app.run(move |cx| {
        gpui_component::init(cx);
        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|cx| MainWindow::new(window, cx, books));
                cx.new(|cx| Root::new(view.into(), window, cx))
            })
        })
        .detach();
    })
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    datapath: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            datapath: "/home/robertd/test/Antiquar".to_string(),
        }
    }
}

fn load_config() -> Config {
    let args = env::args().collect::<Vec<String>>(); // Get command line arguments

    // Try to load config from file
    let config: Config = 'a: {
        if args.get(1).is_some_and(|f| Path::new(f).exists()) {
            // If no argument or an invalid path is passed, use default
            let mut file_content = String::new(); // Buffer for file content

            {
                match {
                    match OpenOptions::new().open(Path::new(args.get(1).unwrap())) { // Try to open file, use default config if it fails
                        v @ Ok(_) => v.unwrap(),
                        v => {
                            eprintln!(
                                "Couldn't open config file: {} \n{}",
                                args.get(1).unwrap(),
                                v.unwrap_err()
                            );
                            let config: Config = Default::default();
                            break 'a config;
                        }
                    }
                }
                .read_to_string(&mut file_content) // Try to read file content, use default config
                {
                    v @ Ok(_) => v.unwrap(),
                    v => {
                        eprintln!(
                            "Couldn't read contents of file: {} \n{}",
                            args.get(1).unwrap(),
                            v.unwrap_err()
                        );
                        let config: Config = Default::default();
                        break 'a config;
                    }
                };
            };

            match toml::from_str(&file_content) {
                v @ Ok(_) => v.unwrap(),
                v => {
                    eprintln!(
                        "Couldn't read contents of file: {} \n{}",
                        args.get(1).unwrap(),
                        v.unwrap_err()
                    );
                    let config: Config = Default::default();
                    break 'a config;
                }
            }
        } else {
            println!("No config file provided, using default values");
            let config: Config = Default::default();
            config
        }
    };

    config
}

fn load_data(data_path: &Path) -> Vec<(u32, Option<Bookdata>, Arc<File>)> {
    if data_path.try_exists().is_err() {
        create_dir_all(data_path).map_err(|e| {
            eprintln!("Failed to create data directory: {}", e);
            println!("Directory creation failed: {}", e)
        });
    }
    let mut filenames: Vec<String> = vec![];
    for entry in WalkDir::new(data_path)
        .max_depth(1)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|e| {
            e.metadata().unwrap().is_file()
                && e.path().extension().map_or(false, |ext| ext == "toml")
        })
    {
        filenames.push(entry.file_name().to_str().unwrap().to_string())
    }
    let filename_regex = Regex::new(r"^\d{5}.toml$").unwrap();
    filenames.retain(|f| filename_regex.is_match(&f.as_bytes()));
    let mut files = vec![];
    for f in filenames {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(data_path.join(f.clone()))
            .unwrap();
        if file.try_lock_exclusive().unwrap() == false {
            eprintln!("Failed to acquire lock for file: {:#?}", data_path.join(f));
            panic!()
        }

        files.push((file, f));
    }
    let mut books: Vec<(u32, Option<Bookdata>, Arc<File>)> = Vec::new();
    for mut file in files {
        let mut content = String::new();
        file.0.read_to_string(&mut content).expect("");
        let deserialized: Result<Bookdata, Error> = toml::from_str(&content);
        match deserialized {
            Ok(_) => {
                books.push((
                    file.1[..5].parse().unwrap(),
                    Option::Some(deserialized.unwrap()),
                    Arc::new(file.0),
                ));
            }
            Err(_) => {
                eprintln!(
                    "Could'nt parse toml file {:#?} with content:\n\n{}",
                    data_path.join(&file.1),
                    content
                );
                books.push((file.1[..5].parse().unwrap(), Option::None, Arc::new(file.0)));
            }
        };
    }
    books
}
