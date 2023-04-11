use clap::{arg, command};
use cursive::event::{Event, Key};
use cursive::theme::{Color, ColorStyle, ColorType, PaletteColor, Theme};
use cursive::utils::markup::StyledString;
use cursive::view::{Nameable, Resizable, Scrollable};
use cursive::views::{
    EditView, Layer, LinearLayout, ScrollView, SelectView, ShadowView, TextView, ViewRef,
};
use cursive::Cursive;
use cursive_extras::{hlayout, vlayout};
use std::cmp::Ordering;
use std::fs::DirEntry;
use std::path::Path;
use std::{env, fs};

const CURR_NAME: &str = "curr";
const PREV_NAME: &str = "prev";
const NEXT_NAME: &str = "next";
const SEARCH_NAME: &str = "search";
const HLAYOUT_NAME: &str = "hlayout";
const PATH_TEXT_NAME: &str = "path_text";

#[derive(Default)]
struct State {
    show_hidden: bool,
    pressed_g: bool,
}

fn update_prev(s: &mut Cursive) {
    let mut prev: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(PREV_NAME).unwrap();
    let prev_select = prev.get_inner_mut();
    if env::current_dir().unwrap().ancestors().count() <= 1 {
        prev_select.clear();
    } else {
        populate_select(
            s,
            prev_select,
            env::current_dir().unwrap().parent().unwrap(),
        );
        update_prev_selection(prev_select);
        prev.scroll_to_important_area();
    }
}

fn update_curr(s: &mut Cursive, is_enter: bool) {
    let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(CURR_NAME).unwrap();
    let curr_select = curr.get_inner_mut();
    let prev: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(PREV_NAME).unwrap();
    let prev_selection = prev.get_inner().selected_id();
    let current_dir = env::current_dir().unwrap();

    if is_enter {
        if fs::read_dir(current_dir).unwrap().count() == 0 {
            return;
        }
        let selection = curr_select.selection().unwrap();
        if selection.path().is_dir() && fs::read_dir(selection.path()).is_ok() {
            env::set_current_dir(selection.path()).unwrap();
        } else {
            return;
        }
    } else if current_dir.ancestors().count() > 1 {
        env::set_current_dir(current_dir.parent().unwrap()).unwrap();
    } else {
        return;
    }

    // now in new dir
    let current_dir = env::current_dir().unwrap();

    if fs::read_dir(&current_dir).unwrap().count() == 0 {
        curr_select.clear();
        return;
    }

    populate_select(s, curr_select, &current_dir);
    if !is_enter {
        curr_select.set_selection(prev_selection.unwrap());
    }

    match curr_select.selection() {
        Some(item) => update_next(s, &item),
        None => {
            let mut next: ViewRef<ScrollView<SelectView<DirEntry>>> =
                s.find_name(NEXT_NAME).unwrap();
            next.get_inner_mut().clear();
        }
    }
}

fn update_next(s: &mut Cursive, item: &DirEntry) {
    let mut hlayout: ViewRef<LinearLayout> = s.find_name(HLAYOUT_NAME).unwrap();
    let mut path_text: ViewRef<TextView> = s.find_name(PATH_TEXT_NAME).unwrap();
    path_text.set_content(format!(
        "{}/{}",
        env::current_dir().unwrap().to_string_lossy(),
        item.file_name().to_string_lossy()
    ));
    hlayout.remove_child(2);
    if item.path().is_dir() {
        let mut next_select = SelectView::<DirEntry>::new()
            .disabled()
            .with_inactive_highlight(false);

        populate_select(s, &mut next_select, &item.path());
        hlayout.add_child(
            ShadowView::new(next_select)
                .top_padding(false)
                .min_width(30),
        );
    }
}

fn update_prev_selection(prev_select: &mut SelectView<DirEntry>) {
    let id = prev_select
        .iter()
        .position(|item| item.1.path().eq(&env::current_dir().unwrap()))
        .unwrap();
    prev_select.set_selection(id);
}

fn read_dir_sorted(path: &Path, show_hidden: bool) -> Vec<DirEntry> {
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut entries = entries
                .flatten()
                .filter(|x| {
                    !x.path().symlink_metadata().unwrap().is_symlink()
                        && (show_hidden || !x.file_name().to_string_lossy().starts_with('.'))
                })
                .collect::<Vec<_>>();
            entries.sort_by(|a, b| {
                let a = a.path();
                let b = b.path();
                let a_name = a.file_name().unwrap().to_string_lossy();
                let b_name = b.file_name().unwrap().to_string_lossy();
                if a.is_dir() && b.is_dir() {
                    a_name.cmp(&b_name)
                } else if a.is_dir() && !b.is_dir() {
                    Ordering::Less
                } else if !a.is_dir() && b.is_dir() {
                    Ordering::Greater
                } else {
                    a_name.cmp(&b_name)
                }
            });
            entries
        }
        Err(_) => vec![],
    }
}

fn populate_select(s: &mut Cursive, select: &mut SelectView<DirEntry>, path: &Path) {
    let show_hidden = s.user_data::<State>().unwrap().show_hidden;
    select.clear();
    let entries = read_dir_sorted(path, show_hidden);
    for e in entries {
        let mut style = ColorStyle::terminal_default();
        if e.path().is_dir() {
            style.front = ColorType::Color(s.current_theme().palette[PaletteColor::Highlight]);
        }
        select.add_item(
            StyledString::styled(e.path().file_name().unwrap().to_string_lossy(), style),
            e,
        );
    }
}

fn submit_search(s: &mut Cursive, text: &str) {
    let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(CURR_NAME).unwrap();
    let curr_select = curr.get_inner_mut();
    let query = text.replace("search: ", "").to_ascii_lowercase();
    let result = curr_select.iter().find(|x| {
        x.0.to_ascii_lowercase().eq(&query) || x.0.to_ascii_lowercase().starts_with(&query)
    });
    if let Some(item) = result {
        let item_id = curr_select.iter().position(|x| x.0.eq(item.0)).unwrap();
        let cb = curr_select.set_selection(item_id);
        cb(s);
        let mut search: ViewRef<EditView> = s.find_name(SEARCH_NAME).unwrap();
        search.set_content("");
        search.disable();
    }
}

fn init(s: &mut Cursive) {
    let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(CURR_NAME).unwrap();
    let curr_select = curr.get_inner_mut();
    populate_select(s, curr_select, &env::current_dir().unwrap());

    let mut prev: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(PREV_NAME).unwrap();
    let prev_select = prev.get_inner_mut();
    populate_select(
        s,
        prev_select,
        env::current_dir().unwrap().parent().unwrap(),
    );
    update_prev_selection(prev_select);
    prev.scroll_to_important_area();

    let curr_selection = curr_select.selection().unwrap();
    update_next(s, &curr_selection);
}

fn handle_exit(s: &mut Cursive) {
    let curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(CURR_NAME).unwrap();
    let selection = curr.get_inner().selection();
    let path = match selection {
        Some(dir) => {
            if dir.metadata().unwrap().is_dir() {
                dir.path()
            } else {
                env::current_dir().unwrap()
            }
        }
        None => env::current_dir().unwrap(),
    }
    .to_str()
    .unwrap()
    .to_owned();
    println!("{}", path);
    s.quit();
}

fn main() {
    let mut siv = cursive::default();
    let mut theme = Theme::terminal_default();
    let matches = command!()
        .arg(arg!(--accent_color [accent_color] "Accent color for seldir"))
        .get_matches();
    let accent_color = if let Some(color) = matches.get_one::<String>("accent_color") {
        Color::parse(color).unwrap()
    } else {
        Color::parse("red").unwrap()
    };
    theme.palette[PaletteColor::Highlight] = accent_color;
    theme.palette[PaletteColor::HighlightInactive] = accent_color;
    siv.set_theme(theme);

    let state = State::default();
    siv.set_user_data(state);

    siv.add_fullscreen_layer(Layer::new(vlayout!(
        TextView::new(env::current_dir().unwrap().to_string_lossy()).with_name(PATH_TEXT_NAME),
        hlayout!(
            ShadowView::new(
                SelectView::<DirEntry>::new()
                    .disabled()
                    .scrollable()
                    .show_scrollbars(false)
                    .with_name(PREV_NAME)
                    .fixed_width(15)
            )
            .top_padding(false)
            .left_padding(false),
            ShadowView::new(
                SelectView::<DirEntry>::new()
                    .on_select(update_next)
                    .scrollable()
                    .show_scrollbars(false)
                    .with_name(CURR_NAME)
                    .min_width(30)
            )
            .top_padding(false)
        )
        .with_name(HLAYOUT_NAME)
        .full_height(),
        EditView::new()
            .disabled()
            .filler(" ")
            .on_submit(submit_search)
            .with_name(SEARCH_NAME)
            .fixed_height(1)
    )));

    init(&mut siv);

    siv.focus_name("curr").unwrap();
    siv.add_global_callback('q', handle_exit);
    siv.add_global_callback(Key::Enter, handle_exit);
    siv.add_global_callback('j', |s| {
        let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(CURR_NAME).unwrap();
        let cb = curr.get_inner_mut().select_down(1);
        cb(s);
        curr.scroll_to_important_area();
    });
    siv.add_global_callback('k', |s| {
        let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(CURR_NAME).unwrap();
        let cb = curr.get_inner_mut().select_up(1);
        cb(s);
        curr.scroll_to_important_area();
    });
    siv.add_global_callback('l', |s| {
        update_curr(s, true);
        update_prev(s);
    });
    siv.add_global_callback('h', |s| {
        update_curr(s, false);
        update_prev(s);
    });
    siv.add_global_callback('G', |s| {
        let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(CURR_NAME).unwrap();
        let curr_select = curr.get_inner_mut();
        let cb = curr_select.set_selection(curr_select.len() - 1);
        cb(s);
        curr.scroll_to_important_area();
    });
    siv.add_global_callback('g', |s| {
        let pressed_g = s.user_data::<State>().unwrap().pressed_g;
        if pressed_g {
            let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> =
                s.find_name(CURR_NAME).unwrap();
            let cb = curr.get_inner_mut().set_selection(0);
            cb(s);
            curr.scroll_to_important_area();
            s.user_data::<State>().unwrap().pressed_g = false;
        } else {
            s.user_data::<State>().unwrap().pressed_g = true;
        }
    });
    siv.add_global_callback(Event::CtrlChar('h'), |s| {
        s.user_data::<State>().unwrap().show_hidden ^= true;
        // TODO fix selection not the same
        init(s);
    });
    siv.add_global_callback('/', |s| {
        let mut search: ViewRef<EditView> = s.find_name(SEARCH_NAME).unwrap();
        let text = "search: ";
        search.set_content(text);
        search.set_cursor(text.len());
        search.enable();
        s.focus_name(SEARCH_NAME).unwrap();
    });
    siv.add_global_callback(Event::Key(Key::Esc), |s| {
        let mut search: ViewRef<EditView> = s.find_name(SEARCH_NAME).unwrap();
        if search.is_enabled() {
            search.set_content("");
            search.disable();
        } else {
            s.quit();
        }
    });

    siv.run();
}
