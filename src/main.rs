use cursive::theme::{BorderStyle, Color, ColorStyle, Theme};
use cursive::utils::markup::StyledString;
use cursive::view::{Margins, Nameable, Resizable};
use cursive::views::{Dialog, Layer, LinearLayout, SelectView, TextView, ViewRef};
use cursive::Cursive;
use std::cmp::Ordering;
use std::env;
use std::fs;
use std::str::FromStr;

struct State {
    show_hidden: bool,
}

impl State {
    fn new() -> State {
        State { show_hidden: false }
    }
}

#[derive(Debug)]
struct File {
    label: String,
    path: String,
    is_dir: bool,
}

impl File {
    fn new(path: String, is_dir: bool) -> File {
        let label = match path.split('/').last() {
            Some(name) => name.to_owned(),
            None => path.clone(),
        };
        File {
            label,
            path,
            is_dir,
        }
    }
}

fn update_prev_curr(s: &mut Cursive, is_enter: bool) {
    let mut curr_dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
    let mut prev_dialog: ViewRef<Dialog> = s.find_name("prev_dialog").unwrap();
    let curr_select = curr_dialog
        .get_content_mut()
        .downcast_mut::<SelectView<File>>()
        .unwrap();
    let prev_select = prev_dialog
        .get_content_mut()
        .downcast_mut::<SelectView<File>>()
        .unwrap();
    let selection = curr_select.selection().unwrap();

    if is_enter {
        if !selection.is_dir {
            return;
        }
        env::set_current_dir(&selection.path).unwrap();
    } else {
        env::set_current_dir("../").unwrap();
    }

    curr_select.clear();
    prev_select.clear();
    populate_select(prev_select, String::from("../"));
    populate_select(curr_select, String::from("./"));
    update_next(s, &curr_select.selection().unwrap());
    update_prev(prev_select);

    let mut path_text: ViewRef<TextView> = s.find_name("path_text").unwrap();
    path_text.set_content(env::current_dir().unwrap().to_str().unwrap());
}

fn update_next(s: &mut Cursive, item: &File) {
    let mut next_dialog: ViewRef<Dialog> = s.find_name("next_dialog").unwrap();
    let next_select = next_dialog
        .get_content_mut()
        .downcast_mut::<SelectView<File>>()
        .unwrap();
    next_select.clear();
    if item.is_dir {
        populate_select(next_select, item.path.clone());
    }
}

fn update_prev(prev_select: &mut SelectView<File>) {
    let id = prev_select
        .iter()
        .position(|item| {
            item.1
                .path
                .eq(env::current_dir().unwrap().to_str().unwrap())
        })
        .unwrap();
    prev_select.set_selection(id);
}

fn read_dir_custom(path: &str) -> Option<Vec<File>> {
    let mut dirs = fs::read_dir(path)
        .ok()?
        .flatten()
        .filter(|x| !fs::symlink_metadata(x.path()).unwrap().is_symlink())
        .map(|x| {
            let path = x.path().to_str().unwrap().to_owned();
            let abs_path = fs::canonicalize(path).unwrap().to_str().unwrap().to_owned();
            let metadata = fs::metadata(x.path()).unwrap();
            File::new(abs_path, metadata.is_dir())
        })
        .collect::<Vec<_>>();
    dirs.sort_by(|a, b| {
        if a.is_dir && b.is_dir {
            a.label.cmp(&b.label)
        } else if a.is_dir && !b.is_dir {
            Ordering::Less
        } else if !a.is_dir && b.is_dir {
            Ordering::Greater
        } else {
            a.label.cmp(&b.label)
        }
    });
    Some(dirs)
}

fn populate_select(select: &mut SelectView<File>, path: String) {
    match read_dir_custom(&path) {
        Some(files) => {
            for file in files {
                let mut style = ColorStyle::terminal_default();
                if file.is_dir {
                    style.front = cursive::theme::ColorType::Color(Color::from_str("red").unwrap());
                }
                select.add_item(StyledString::styled(file.label.clone(), style), file);
            }
        }
        None => {
            let dummy_item = File {
                label: "...".to_owned(),
                path: "".to_owned(),
                is_dir: false,
            };
            select.add_item(dummy_item.label.clone(), dummy_item);
        }
    }
}

fn main() {
    let mut siv = cursive::default();
    let mut theme = Theme::terminal_default();
    theme.borders = BorderStyle::None;
    theme
        .palette
        .set_color("Highlight", Color::from_str("red").unwrap());
    theme
        .palette
        .set_color("HighlightInactive", Color::from_str("red").unwrap());
    siv.set_theme(theme);

    let state = State::new();
    siv.set_user_data(state);

    let mut prev_select = SelectView::<File>::new().disabled();
    let mut curr_select = SelectView::<File>::new();
    let mut next_select = SelectView::<File>::new()
        .disabled()
        .with_inactive_highlight(false);

    populate_select(&mut prev_select, String::from("../"));
    populate_select(&mut curr_select, String::from("./"));
    update_prev(&mut prev_select);

    let curr_selection = curr_select.selection().unwrap();
    if curr_selection.is_dir {
        populate_select(&mut next_select, curr_selection.path.clone())
    }

    curr_select.set_on_select(update_next);

    siv.add_fullscreen_layer(Layer::new(
        LinearLayout::vertical()
            .child(
                TextView::new(env::current_dir().unwrap().to_string_lossy()).with_name("path_text"),
            )
            .child(
                LinearLayout::horizontal()
                    .child(
                        Dialog::new()
                            .padding(Margins::zeroes())
                            .content(prev_select)
                            .with_name("prev_dialog")
                            .full_height()
                            .fixed_width(20),
                    )
                    .child(
                        Dialog::new()
                            .padding(Margins::zeroes())
                            .content(curr_select)
                            .with_name("curr_dialog")
                            .full_width(),
                    )
                    .child(
                        Dialog::new()
                            .padding(Margins::zeroes())
                            .content(next_select)
                            .with_name("next_dialog")
                            .full_screen(),
                    ),
            ),
    ));

    siv.focus_name("curr_dialog").unwrap();
    siv.add_global_callback('q', |s| {
        let cwd = env::current_dir().unwrap().to_str().unwrap().to_owned();
        println!("{}", cwd);
        s.quit();
    });
    siv.add_global_callback('j', |s| {
        let mut curr_dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
        let curr_select = curr_dialog
            .get_content_mut()
            .downcast_mut::<SelectView<File>>()
            .unwrap();
        let cb = curr_select.select_down(1);
        cb(s);
    });
    siv.add_global_callback('k', |s| {
        let mut curr_dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
        let curr_select = curr_dialog
            .get_content_mut()
            .downcast_mut::<SelectView<File>>()
            .unwrap();
        let cb = curr_select.select_up(1);
        cb(s);
    });
    siv.add_global_callback('l', |s| {
        update_prev_curr(s, true);
    });
    siv.add_global_callback('h', |s| {
        update_prev_curr(s, false);
    });
    siv.add_global_callback('G', |s| {
        let mut curr_dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
        let curr_select = curr_dialog
            .get_content_mut()
            .downcast_mut::<SelectView<File>>()
            .unwrap();
        let cb = curr_select.set_selection(curr_select.len() - 1);
        cb(s);
    });
    siv.add_global_callback('g', |s| {
        let mut curr_dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
        let curr_select = curr_dialog
            .get_content_mut()
            .downcast_mut::<SelectView<File>>()
            .unwrap();
        let cb = curr_select.set_selection(0);
        cb(s);
    });
    siv.add_global_callback('t', |s| {
        s.user_data::<State>().unwrap().show_hidden ^= true;
    });

    siv.run();
}
