use cursive::event::{Event, Key};
use cursive::theme::{BorderStyle, Color, ColorStyle, Theme};
use cursive::utils::markup::StyledString;
use cursive::view::{Margins, Nameable, Resizable, Scrollable};
use cursive::views::{Dialog, EditView, Layer, LinearLayout, SelectView, TextView, ViewRef, ScrollView};
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

struct File {
    label: String,
    path: String,
    is_dir: bool,
}

impl File {
    fn new(label: String, path: String, is_dir: bool) -> File {
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
        .downcast_mut::<ScrollView<SelectView<File>>>()
        .unwrap()
        .get_inner_mut();
    let prev_scroll_view = prev_dialog
        .get_content_mut()
        .downcast_mut::<ScrollView<SelectView<File>>>()
        .unwrap();
    let prev_select = prev_scroll_view.get_inner_mut();

    if is_enter {
        let selection = curr_select.selection().unwrap();
        if selection.is_dir {
            env::set_current_dir(&selection.path).unwrap();
        } else {
            return;
        }
    } else {
        env::set_current_dir("../").unwrap();
    }

    let show_hidden = s.user_data::<State>().unwrap().show_hidden;

    populate_select(prev_select, String::from("../"), show_hidden);
    populate_select(curr_select, String::from("./"), show_hidden);
    update_next(s, &curr_select.selection().unwrap());
    update_prev(prev_select);
    prev_scroll_view.scroll_to_important_area();

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
        let show_hidden = s.user_data::<State>().unwrap().show_hidden;
        populate_select(next_select, item.path.clone(), show_hidden);
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

fn read_dir_custom(path: &str, show_hidden: bool) -> Option<Vec<File>> {
    let mut dirs = fs::read_dir(path)
        .ok()?
        .flatten()
        .filter(|x| {
            !fs::symlink_metadata(x.path()).unwrap().is_symlink()
                && (show_hidden || !x.file_name().to_string_lossy().starts_with('.'))
        })
        .map(|x| {
            let path = x.path().to_str().unwrap().to_owned();
            let abs_path = fs::canonicalize(path).unwrap().to_str().unwrap().to_owned();
            let is_dir = x.metadata().unwrap().is_dir();
            let label = x.file_name().into_string().unwrap();
            File::new(label, abs_path, is_dir)
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
    if dirs.is_empty() {
        None
    } else {
        Some(dirs)
    }
}

fn populate_select(select: &mut SelectView<File>, path: String, show_hidden: bool) {
    select.clear();
    match read_dir_custom(&path, show_hidden) {
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

fn submit_search(s: &mut Cursive, text: &str) {
    let mut curr_dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
    let curr_select = curr_dialog
        .get_content_mut()
        .downcast_mut::<ScrollView<SelectView<File>>>()
        .unwrap()
        .get_inner_mut();
    let query = text.replace("search: ", "").to_ascii_lowercase();
    let result = curr_select.iter()
        .find(|x| x.0.to_ascii_lowercase().eq(&query) || x.0.starts_with(&query));
    if let Some(item) = result {
        let item_id = curr_select.iter().position(|x| x.0.eq(item.0)).unwrap();
        let cb = curr_select.set_selection(item_id);
        cb(s);
        let mut search: ViewRef<EditView> = s.find_name("search").unwrap();
        search.set_content("");
        search.disable();
    }
}

fn init(s: &mut Cursive) {
    let show_hidden = s.user_data::<State>().unwrap().show_hidden;
    let mut curr_dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
    let curr_select = curr_dialog
        .get_content_mut()
        .downcast_mut::<ScrollView<SelectView<File>>>()
        .unwrap()
        .get_inner_mut();
    populate_select(curr_select, String::from("./"), show_hidden);

    let mut prev_dialog: ViewRef<Dialog> = s.find_name("prev_dialog").unwrap();
    let prev_scroll_view = prev_dialog
        .get_content_mut()
        .downcast_mut::<ScrollView<SelectView<File>>>()
        .unwrap();
    let prev_select = prev_scroll_view.get_inner_mut();
    populate_select(prev_select, String::from("../"), show_hidden);
    update_prev(prev_select);
    prev_scroll_view.scroll_to_important_area();

    let mut next_dialog: ViewRef<Dialog> = s.find_name("next_dialog").unwrap();
    let next_select = next_dialog
        .get_content_mut()
        .downcast_mut::<SelectView<File>>()
        .unwrap();

    let curr_selection = curr_select.selection().unwrap();
    if curr_selection.is_dir {
        populate_select(next_select, curr_selection.path.clone(), show_hidden)
    }
}

fn main() {
    let mut siv = cursive::default();
    let mut theme = Theme::terminal_default();
    theme.borders = BorderStyle::None;
    theme.palette
        .set_color("Highlight", Color::from_str("red").unwrap());
    theme.palette
        .set_color("HighlightInactive", Color::from_str("red").unwrap());
    siv.set_theme(theme);

    let state = State::new();
    siv.set_user_data(state);

    let prev_select = SelectView::<File>::new()
        .disabled()
        .scrollable()
        .show_scrollbars(false);
    let curr_select = SelectView::<File>::new()
        .on_select(update_next)
        .scrollable()
        .show_scrollbars(false);
    let next_select = SelectView::<File>::new()
        .disabled()
        .with_inactive_highlight(false);

    siv.add_fullscreen_layer(Layer::new(
        LinearLayout::vertical()
            .child(
                TextView::new(env::current_dir().unwrap().to_string_lossy())
                    .with_name("path_text"),
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
            )
            .child(
                EditView::new()
                    .disabled()
                    .filler(" ")
                    .on_submit(submit_search)
                    // .style(ColorStyle::new(BaseColor::White, BaseColor::Black))
                    .with_name("search")
                    .fixed_height(1),
            ),
    ));

    init(&mut siv);

    siv.focus_name("curr_dialog").unwrap();
    siv.add_global_callback('q', |s| {
        let cwd = env::current_dir().unwrap().to_str().unwrap().to_owned();
        println!("{}", cwd);
        s.quit();
    });
    siv.add_global_callback('j', |s| {
        let mut dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
        let scroll_view = dialog
            .get_content_mut()
            .downcast_mut::<ScrollView<SelectView<File>>>()
            .unwrap();
        let select = scroll_view.get_inner_mut();
        let cb = select.select_down(1);
        cb(s);
        scroll_view.scroll_to_important_area();
    });
    siv.add_global_callback('k', |s| {
        let mut dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
        let scroll_view = dialog
            .get_content_mut()
            .downcast_mut::<ScrollView<SelectView<File>>>()
            .unwrap();
        let select = scroll_view.get_inner_mut();
        let cb = select.select_up(1);
        cb(s);
        scroll_view.scroll_to_important_area();
    });
    siv.add_global_callback('l', |s| {
        update_prev_curr(s, true);
    });
    siv.add_global_callback('h', |s| {
        update_prev_curr(s, false);
    });
    siv.add_global_callback('G', |s| {
        let mut dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
        let scroll_view = dialog
            .get_content_mut()
            .downcast_mut::<ScrollView<SelectView<File>>>()
            .unwrap();
        let select = scroll_view.get_inner_mut();
        let cb = select.set_selection(select.len() - 1);
        cb(s);
        scroll_view.scroll_to_important_area();
    });
    siv.add_global_callback('g', |s| {
        let mut dialog: ViewRef<Dialog> = s.find_name("curr_dialog").unwrap();
        let scroll_view = dialog
            .get_content_mut()
            .downcast_mut::<ScrollView<SelectView<File>>>()
            .unwrap();
        let select = scroll_view.get_inner_mut();
        let cb = select.set_selection(0);
        cb(s);
        scroll_view.scroll_to_important_area();
    });
    siv.add_global_callback(Event::CtrlChar('h'), |s| {
        s.user_data::<State>().unwrap().show_hidden ^= true;
        init(s);
    });
    siv.add_global_callback('/', |s| {
        let mut search: ViewRef<EditView> = s.find_name("search").unwrap();
        let text = "search: ";
        search.set_content(text);
        search.set_cursor(text.len());
        search.enable();
        s.focus_name("search").unwrap();
    });
    siv.add_global_callback(Event::Key(Key::Esc), |s| {
        let mut search: ViewRef<EditView> = s.find_name("search").unwrap();
        search.set_content("");
        search.disable();
    });

    siv.run();
}
