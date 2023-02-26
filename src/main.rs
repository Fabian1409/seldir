use cursive::Cursive;
use cursive::theme::{Theme, BorderStyle, Style, ColorStyle, ColorType, Color, PaletteColor};
use cursive::utils::markup::StyledString;
use cursive::view::{Nameable, Resizable, Scrollable, ScrollStrategy};
use cursive::views::{Dialog, SelectView, LinearLayout, Layer, ViewRef, TextView, ListView, TextContent};
use std::cmp::Ordering;
use std::io::Write;
use std::str::FromStr;
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

fn update_selects(s: &mut Cursive, is_enter: bool) {
    let mut curr_dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
    let mut prev_dialog: ViewRef<Dialog> = s.find_name("prev_dialog").unwrap();
    let mut curr_select = curr_dialog.get_content_mut().downcast_mut::<SelectView<Item>>().unwrap();
    let mut prev_select = prev_dialog.get_content_mut().downcast_mut::<SelectView<Item>>().unwrap();
    let selection = curr_select.selection().unwrap().path.clone();

    if is_enter {
        env::set_current_dir(&selection).expect("failed to set dir");
    } else {
        env::set_current_dir("../").expect("failed to set dir");
    }

    curr_select.clear();
    prev_select.clear();
    populate_select(&mut prev_select, String::from("../"));
    populate_select(&mut curr_select, String::from("./"));

    update_next(s, &curr_select.selection().unwrap());

    let prev_selection_id;
    if is_enter {
        prev_selection_id = prev_select.iter().position(|item| item.1.path.contains(&selection)).unwrap();
    } else {
        let cwd = env::current_dir().unwrap().to_str().unwrap().to_owned();
        let parent_dir = cwd.split("/").last().unwrap();
        prev_selection_id = prev_select.iter().position(|item| item.1.path.contains(parent_dir)).unwrap();
    }
    prev_select.set_selection(prev_selection_id);
}

fn update_next(s: &mut Cursive, item: &Item) {
    let mut next_dialog: ViewRef<Dialog> = s.find_name("next_dialog").unwrap();
    let next_select = next_dialog.get_content_mut().downcast_mut::<SelectView<Item>>().unwrap();
    next_select.clear();
    if item.is_dir {
        populate_select(next_select, item.path.clone());
    }
}

fn populate_select(select: &mut SelectView<Item>, path: String) {
    let read_dir = fs::read_dir(path.clone()).unwrap();
    let mut dirs: Vec<String> = read_dir.map(|x| x.unwrap().path().to_str().unwrap().to_owned()).collect();
    dirs.sort_by(|a, b| {
        let a_meta = fs::metadata(a).unwrap();
        let b_meta = fs::metadata(b).unwrap();
        if a_meta.is_dir() && b_meta.is_dir() {
            a.cmp(b)
        } else if a_meta.is_dir() && !b_meta.is_dir() {
            Ordering::Less            
        } else if !a_meta.is_dir() && b_meta.is_dir() {
            Ordering::Greater
        } else {
            a.cmp(b)
        }
    });

    for dir in dirs {
        let label_str = dir.replace(&path, "").replace("/", "");
        let item = Item::new(dir);
        let mut style = ColorStyle::terminal_default();
        if item.is_dir {
            style.front = cursive::theme::ColorType::Color(Color::from_str("red").unwrap());
        } 
        let label = StyledString::styled(label_str, style);
        select.add_item(label, item);
    }
}

fn main() {
    let mut siv = cursive::default();
    let mut theme = Theme::terminal_default();
    theme.borders = BorderStyle::None;
    theme.palette.set_color("Highlight", Color::from_str("red").unwrap());
    theme.palette.set_color("HighlightInactive", Color::from_str("red").unwrap());
    siv.set_theme(theme);

    let mut prev_select = SelectView::<Item>::new().disabled();
    let mut curr_select = SelectView::<Item>::new();
    let mut next_select = SelectView::<Item>::new().disabled();

    next_select.set_inactive_highlight(false);

    populate_select(&mut prev_select, String::from("../"));
    populate_select(&mut curr_select, String::from("./"));

    let cwd = env::current_dir().unwrap().to_str().unwrap().to_owned();
    let parent_dir = cwd.split("/").last().unwrap();
    let prev_selection_id = prev_select.iter().position(|item| item.1.path.contains(parent_dir)).unwrap();
    prev_select.set_selection(prev_selection_id);

    let curr_selection = curr_select.selection().unwrap();
    if curr_selection.is_dir {
       populate_select(&mut next_select, curr_selection.path.clone()) 
    }

    curr_select.set_on_select(|s, item| update_next(s, item));

    siv.add_layer(
        Layer::new(
            LinearLayout::horizontal()
                .child(
                    Dialog::new()
                        .content(prev_select)
                        .with_name("prev_dialog")
                        .full_height()
                        .fixed_width(20)
                )
                .child(
                    Dialog::new()
                        .content(curr_select)
                        .with_name("curr_dialog")
                        .scrollable()
                        .show_scrollbars(false)
                        .full_height()
                        .full_width()
                )
                .child(
                    Dialog::new()
                        .content(next_select)
                        .with_name("next_dialog")
                        .full_height()
                        .full_width()
                )
        ),
    );

    siv.focus_name("curr_dialog").unwrap();
    siv.add_global_callback('q', |s| {
        let cwd = env::current_dir().unwrap().to_str().unwrap().to_owned();
        let mut file = File::create(format!("/tmp/seldir")).expect("failed to create tmp file");
        file.write_all(cwd.as_bytes()).expect("failed writing to file");
        s.quit();
    });
    siv.add_global_callback('j', |s| {
        let mut curr_dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
        let curr_select = curr_dialog.get_content_mut().downcast_mut::<SelectView<Item>>().unwrap();
        let cb = curr_select.select_down(1);
        cb(s);
    });
    siv.add_global_callback('k', |s| {
        let mut curr_dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
        let curr_select = curr_dialog.get_content_mut().downcast_mut::<SelectView<Item>>().unwrap();
        let cb = curr_select.select_up(1);
        cb(s);
    });
    siv.add_global_callback('l', |s| {
        update_selects(s, true);
    });
    siv.add_global_callback('h', |s| {
        update_selects(s, false);
    });
    siv.add_global_callback('G', |s| {
        let mut curr_dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
        let curr_select = curr_dialog.get_content_mut().downcast_mut::<SelectView<Item>>().unwrap();
        let cb = curr_select.set_selection(curr_select.len() - 1);
        cb(s);
    });
    siv.add_global_callback('g', |s| {
        //s.set_user_data()
        let mut curr_dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
        let curr_select = curr_dialog.get_content_mut().downcast_mut::<SelectView<Item>>().unwrap();
        let cb = curr_select.set_selection(0);
        cb(s);
    });

    siv.run();
}