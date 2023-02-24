use cursive::event::EventResult;
use cursive::theme::{Theme, BorderStyle};
use cursive::view::{Nameable, Resizable};
use cursive::views::{Dialog, SelectView, LinearLayout, Layer};
use std::io::Write;
use std::{fs, fs::File};
use std::env;

struct Item {
    path: String,
    is_dir: bool,
}

impl Item {
    fn new(path: String) -> Item {
        let metadata = fs::metadata(path.clone()).unwrap();
        let is_dir = metadata.is_dir();
        Item { path, is_dir }
    }
}

fn populate_select(select: &mut SelectView<Item>, path: String) {
    let dirs = fs::read_dir(path.clone()).unwrap();
    for dir in dirs {
        let dir_str = dir.unwrap().path().to_str().unwrap().to_owned();
        let item = Item::new(dir_str.clone());
        select.add_item(dir_str.replace(&path, ""), item);
    }
}

fn main() {
    let mut siv = cursive::default();
    let mut theme = Theme::terminal_default();
    theme.borders = BorderStyle::None;
    siv.set_theme(theme);

    let mut select_prev = SelectView::<Item>::new().with_name("prev");
    let mut select_next = SelectView::<Item>::new().with_name("next");
    let mut select_curr = SelectView::<Item>::new().with_name("curr");

    populate_select(&mut select_prev.get_mut(), String::from("../"));
    populate_select(&mut select_curr.get_mut(), String::from("./"));
    populate_select(&mut select_next.get_mut(), String::from("./"));

    let mut layer = Layer::new(LinearLayout::horizontal());
    layer.get_inner_mut().add_child(Dialog::around(select_prev).title("prev").full_height().full_width());
    layer.get_inner_mut().add_child(Dialog::around(select_curr).title("curr").full_height().full_width());
    layer.get_inner_mut().add_child(Dialog::around(select_next).title("next").full_height().full_width());

    siv.add_layer(layer);
    siv.focus_name("curr").expect("curr not found");
    siv.add_global_callback('q', |s| {
        let cwd = env::current_dir().unwrap().to_str().expect("failed to get cwd").to_owned();
        let mut file = File::create(format!("/tmp/seldir")).expect("failed to create tmp file");
        file.write_all(cwd.as_bytes()).expect("failed writing to file");
        s.quit();
    });
    siv.run();
}