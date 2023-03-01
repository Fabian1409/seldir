use cursive::Cursive;
use cursive::theme::{Theme, BorderStyle, ColorStyle, Color};
use cursive::utils::markup::StyledString;
use cursive::view::{Nameable, Resizable, Scrollable, Margins};
use cursive::views::{Dialog, SelectView, LinearLayout, Layer, ViewRef, TextView};
use std::cmp::Ordering;
use std::str::FromStr;
use std::fs;
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
    let curr_select = curr_dialog.get_content_mut().downcast_mut::<SelectView<Item>>().unwrap();
    let prev_select = prev_dialog.get_content_mut().downcast_mut::<SelectView<Item>>().unwrap();
    let selection = curr_select.selection().unwrap();

    if !selection.is_dir && is_enter {
        return;
    }

    if (is_enter && env::set_current_dir(&selection.path).is_err()) ||
        env::set_current_dir("../").is_err() {
        return;
    }

    curr_select.clear();
    prev_select.clear();
    populate_select(prev_select, String::from("../"));
    populate_select(curr_select, String::from("./"));
    update_next(s, &curr_select.selection().unwrap());

    if is_enter {
        let id = prev_select.iter().position(|item| item.1.path.contains(&selection.path)).unwrap();
        prev_select.set_selection(id);
    } else {
        update_prev_selection(prev_select);
    };

    let mut path_text: ViewRef<TextView> = s.find_name("path_text").unwrap();
    path_text.set_content(env::current_dir().unwrap().to_str().unwrap());
}

fn update_next(s: &mut Cursive, item: &Item) {
    let mut next_dialog: ViewRef<Dialog> = s.find_name("next_dialog").unwrap();
    let next_select = next_dialog.get_content_mut().downcast_mut::<SelectView<Item>>().unwrap();
    next_select.clear();
    if item.is_dir {
        populate_select(next_select, item.path.clone());
    }
}

fn update_prev_selection(prev_select: &mut SelectView<Item>) {
    let cwd = env::current_dir().unwrap().to_str().unwrap().to_owned();
    let parent_dir = cwd.split('/').last().unwrap();
    let id = prev_select.iter().position(|item| item.1.path.contains(parent_dir)).unwrap();
    prev_select.set_selection(id);
}

fn read_dir_custom(path: &str) -> Option<Vec<String>> {
    let read_dir = fs::read_dir(path);
    if read_dir.is_err() {
        return None;
    }
    let mut dirs: Vec<String> = fs::read_dir(path).unwrap()
        .filter_map(|x| x.ok())
        .map(|x| x.path().to_string_lossy().into_owned())
        .filter(|x| 
            !fs::symlink_metadata(x).unwrap().is_symlink() &&
            !x.split('/').last().unwrap().starts_with('.'))
        .collect();

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
    Some(dirs)
}

fn populate_select(select: &mut SelectView<Item>, path: String) {
    match read_dir_custom(&path) {
        Some(dirs) => {
                for dir in dirs {
                let label_str = dir.replace(&path, "").replace('/', "");
                let item = Item::new(dir);
                let mut style = ColorStyle::terminal_default();
                if item.is_dir {
                    style.front = cursive::theme::ColorType::Color(Color::from_str("red").unwrap());
                } 
                select.add_item(StyledString::styled(label_str, style), item);
            }
        },
        None => {
            let dummy_item = Item { path: "".to_owned(), is_dir: false };
            select.add_item("...", dummy_item);
        }
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
    let mut next_select = SelectView::<Item>::new().disabled().with_inactive_highlight(false);

    populate_select(&mut prev_select, String::from("../"));
    populate_select(&mut curr_select, String::from("./"));

    update_prev_selection(&mut prev_select);

    let curr_selection = curr_select.selection().unwrap();
    if curr_selection.is_dir {
       populate_select(&mut next_select, curr_selection.path.clone()) 
    }

    curr_select.set_on_select(update_next);

    siv.add_layer(
        Layer::new(
            LinearLayout::vertical()
                .child(
                    TextView::new(
                        env::current_dir().unwrap()
                            .to_string_lossy()
                    ).with_name("path_text")
                )
                .child(
                LinearLayout::horizontal()
                    .child(
                        Dialog::new()
                            .padding(Margins::zeroes())
                            .content(prev_select)
                            .with_name("prev_dialog")
                            .full_height()
                            .fixed_width(20)
                    )
                    .child(
                        Dialog::new()
                            .padding(Margins::zeroes())
                            .content(curr_select)
                            .with_name("curr_dialog")
                            .scrollable()
                            .show_scrollbars(false)
                            .full_height()
                            .full_width()
                    )
                    .child(
                        Dialog::new()
                            .padding(Margins::zeroes())
                            .content(next_select)
                            .with_name("next_dialog")
                            .full_height()
                            .full_width()
                    )
                )
        ),
    );

    siv.focus_name("curr_dialog").unwrap();
    siv.add_global_callback('q', |s| {
        let cwd = env::current_dir().unwrap().to_str().unwrap().to_owned();
        println!("{}", cwd);
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
        if s.user_data::<bool>().is_none() {
            s.set_user_data(true);
        } else {
            s.take_user_data::<bool>().unwrap();
            let mut curr_dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
            let curr_select = curr_dialog.get_content_mut().downcast_mut::<SelectView<Item>>().unwrap();
            let cb = curr_select.set_selection(0);
            cb(s);
        }
    });

    siv.run();
}