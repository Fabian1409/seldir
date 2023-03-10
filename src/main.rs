use cursive::event::{Event, Key};
use cursive::theme::{Color, ColorStyle, Theme, PaletteColor, BaseColor, ColorType};
use cursive::utils::markup::StyledString;
use cursive::view::{Nameable, Resizable, Scrollable};
use cursive::views::{EditView, Layer, LinearLayout, SelectView, TextView, ViewRef, ScrollView, ShadowView};
use cursive::Cursive;
use std::cmp::Ordering;
use std::path::Path;
use std::{env, fs};
use std::fs::DirEntry;

struct State {
    show_hidden: bool,
}

impl State {
    fn new() -> State {
        State { show_hidden: false }
    }
}

fn update_prev_curr(s: &mut Cursive, is_enter: bool) {
    let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name("curr").unwrap();
    let curr_select = curr.get_inner_mut();
    let mut prev: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name("prev").unwrap();
    let prev_select = prev.get_inner_mut();

    if is_enter {
        let selection = curr_select.selection().unwrap();
        if selection.path().is_dir() && fs::read_dir(selection.path()).is_ok() { // TODO handle empty dirs
            env::set_current_dir(selection.path()).unwrap();
        } else {
            return;
        }
    } else if env::current_dir().unwrap().ancestors().count() > 2 { // TODO handle going back to /
        env::set_current_dir(env::current_dir().unwrap().parent().unwrap()).unwrap()
    } else {
        return;
    }

    let show_hidden = s.user_data::<State>().unwrap().show_hidden;

    populate_select(prev_select, env::current_dir().unwrap().parent().unwrap(), show_hidden);
    populate_select(curr_select, &env::current_dir().unwrap(), show_hidden);
    update_next(s, &curr_select.selection().unwrap());
    update_prev(prev_select);
    prev.scroll_to_important_area();

    let mut path_text: ViewRef<TextView> = s.find_name("path_text").unwrap();
    path_text.set_content(env::current_dir().unwrap().to_str().unwrap());
}

fn update_next(s: &mut Cursive, item: &DirEntry) {
    let mut next_select: ViewRef<SelectView<DirEntry>> = s.find_name("next").unwrap();
    next_select.clear();
    if item.path().is_dir() {
        let show_hidden = s.user_data::<State>().unwrap().show_hidden;
        populate_select(&mut next_select, &item.path(), show_hidden);
    }
}

fn update_prev(prev_select: &mut SelectView<DirEntry>) {
    let id = prev_select
        .iter()
        .position(|item| {
            item.1.path().eq(&env::current_dir().unwrap())
        })
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
        },
        Err(_) => vec![]
    }
}

fn populate_select(select: &mut SelectView<DirEntry>, path: &Path, show_hidden: bool) {
    select.clear();
    let entries = read_dir_sorted(path, show_hidden);
    for e in entries {
        let mut style = ColorStyle::terminal_default();
        if e.path().is_dir() {
            style.front = ColorType::Color(Color::Dark(BaseColor::Red));
        }
        select.add_item(StyledString::styled(e.path().file_name().unwrap().to_string_lossy(), style), e);
    }
}

fn submit_search(s: &mut Cursive, text: &str) {
    let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name("curr").unwrap();
    let curr_select = curr.get_inner_mut();
    let query = text.replace("search: ", "").to_ascii_lowercase();
    let result = curr_select.iter()
        .find(|x| x.0.to_ascii_lowercase().eq(&query) || x.0.to_ascii_lowercase().starts_with(&query));
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
    let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name("curr").unwrap();
    let curr_select = curr.get_inner_mut();
    populate_select(curr_select, &env::current_dir().unwrap(), show_hidden);

    let mut prev: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name("prev").unwrap();
    let prev_select = prev.get_inner_mut();
    populate_select(prev_select, env::current_dir().unwrap().parent().unwrap(), show_hidden);
    update_prev(prev_select);
    prev.scroll_to_important_area();

    let mut next_select: ViewRef<SelectView<DirEntry>> = s.find_name("next").unwrap();
    let curr_selection = curr.get_inner().selection().unwrap();
    if curr_selection.path().is_dir() {
        populate_select(&mut next_select, &curr_selection.path(), show_hidden)
    }
}

fn main() {
    let mut siv = cursive::default();

    let mut theme = Theme::terminal_default();
    theme.palette[PaletteColor::Highlight] = Color::Dark(BaseColor::Red);
    theme.palette[PaletteColor::HighlightInactive] = Color::Dark(BaseColor::Red);
    siv.set_theme(theme);

    let state = State::new();
    siv.set_user_data(state);

    siv.add_fullscreen_layer(Layer::new(
        LinearLayout::vertical()
            .child(
                TextView::new(env::current_dir().unwrap().to_string_lossy())
                    .with_name("path_text"),
            )
            .child(
                LinearLayout::horizontal()
                    .child(
                        ShadowView::new(
                            SelectView::<DirEntry>::new()
                                .disabled()
                                .scrollable()
                                .show_scrollbars(false)
                                .with_name("prev")
                                .full_height()
                                .fixed_width(20)
                            ).top_padding(false).left_padding(false),
                    )
                    .child(
                        ShadowView::new(
                            SelectView::<DirEntry>::new()
                                .on_select(update_next)
                                .scrollable()
                                .show_scrollbars(false)
                                .with_name("curr")
                                .full_width()
                            ).top_padding(false),
                    )
                    .child(
                        ShadowView::new(
                            SelectView::<DirEntry>::new()
                                .disabled()
                                .with_inactive_highlight(false)
                                .with_name("next")
                                .full_screen()
                            ).top_padding(false),
                    )
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

    siv.focus_name("curr").unwrap();
    siv.add_global_callback('q', |s| {
        let cwd = env::current_dir().unwrap().to_str().unwrap().to_owned();
        println!("{}", cwd);
        s.quit();
    });
    siv.add_global_callback('j', |s| {
        let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name("curr").unwrap();
        let cb = curr.get_inner_mut().select_down(1);
        cb(s);
        curr.scroll_to_important_area();
    });
    siv.add_global_callback('k', |s| {
        let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name("curr").unwrap();
        let cb = curr.get_inner_mut().select_up(1);
        cb(s);
        curr.scroll_to_important_area();
    });
    siv.add_global_callback('l', |s| {
        update_prev_curr(s, true);
    });
    siv.add_global_callback('h', |s| {
        update_prev_curr(s, false);
    });
    siv.add_global_callback('G', |s| {
        let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name("curr").unwrap();
        let curr_select = curr.get_inner_mut();
        let cb = curr_select.set_selection(curr_select.len() - 1);
        cb(s);
        curr.scroll_to_important_area();
    });
    siv.add_global_callback('g', |s| {
        let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name("curr").unwrap();
        let cb = curr.get_inner_mut().set_selection(0);
        cb(s);
        curr.scroll_to_important_area();
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
