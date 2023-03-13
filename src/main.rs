use cursive::event::{Event, Key};
use cursive::theme::{BaseColor, Color, ColorStyle, ColorType, PaletteColor, Theme};
use cursive::utils::markup::StyledString;
use cursive::view::{Nameable, Resizable, Scrollable};
use cursive::views::{
    EditView, Layer, LinearLayout, ScrollView, SelectView, ShadowView, TextView, ViewRef,
};
use cursive::Cursive;
use cursive_extras::{hlayout, vlayout, ImageView};
use pdf_extract::extract_text;
use std::cmp::Ordering;
use std::fs::DirEntry;
use std::path::Path;
use std::{env, fs};

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

    let curr_selection = prev_select.selected_id().unwrap();

    if is_enter {
        let selection = curr_select.selection().unwrap();
        if selection.path().is_dir() && fs::read_dir(selection.path()).is_ok() {
            // TODO handle empty dirs
            env::set_current_dir(selection.path()).unwrap();
        } else {
            return;
        }
    } else if env::current_dir().unwrap().ancestors().count() > 2 {
        // TODO handle going back to /
        env::set_current_dir(env::current_dir().unwrap().parent().unwrap()).unwrap()
    } else {
        return;
    }

    let show_hidden = s.user_data::<State>().unwrap().show_hidden;

    populate_select(
        prev_select,
        env::current_dir().unwrap().parent().unwrap(),
        show_hidden,
    );

    populate_select(curr_select, &env::current_dir().unwrap(), show_hidden);
    if !is_enter {
        curr_select.set_selection(curr_selection);
    }

    update_next(s, &curr_select.selection().unwrap());
    update_prev(prev_select);
    prev.scroll_to_important_area();

    let mut path_text: ViewRef<TextView> = s.find_name("path_text").unwrap();
    path_text.set_content(env::current_dir().unwrap().to_str().unwrap());
}

fn update_next(s: &mut Cursive, item: &DirEntry) {
    let mut hlayout: ViewRef<LinearLayout> = s.find_name("hlayout").unwrap();
    hlayout.remove_child(2);
    if item.path().is_dir() {
        let mut next_select = SelectView::<DirEntry>::new()
            .disabled()
            .with_inactive_highlight(false);

        let show_hidden = s.user_data::<State>().unwrap().show_hidden;
        populate_select(&mut next_select, &item.path(), show_hidden);
        hlayout.add_child(
            ShadowView::new(next_select)
                .top_padding(false)
                .min_width(30),
        );
    } else {
        let name = item.file_name();
        let parts = name.to_str().unwrap().split('.').collect::<Vec<&str>>();
        match parts[..] {
            [_, "pdf"] => {
                hlayout.add_child(
                    ShadowView::new(
                        TextView::new(extract_text(item.path()).unwrap_or("pdf".to_owned()))
                            .min_width(50),
                    )
                    .top_padding(false),
                );
            }
            [_, "png" | "jpg" | "jpeg"] => {
                hlayout.add_child(
                    ShadowView::new(
                        ImageView::new(50, 18)
                            .image(&item.path().to_string_lossy())
                            .min_width(50),
                    )
                    .top_padding(false),
                );
            }
            [_, _] | [_] => {
                hlayout.add_child(
                    ShadowView::new(
                        TextView::new(
                            fs::read_to_string(item.path()).unwrap_or("content".to_owned()),
                        )
                        .min_width(50),
                    )
                    .top_padding(false),
                );
            }
            _ => todo!(),
        };
    }
}

fn update_prev(prev_select: &mut SelectView<DirEntry>) {
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

fn populate_select(select: &mut SelectView<DirEntry>, path: &Path, show_hidden: bool) {
    select.clear();
    let entries = read_dir_sorted(path, show_hidden);
    for e in entries {
        let mut style = ColorStyle::terminal_default();
        if e.path().is_dir() {
            style.front = ColorType::Color(Color::Dark(BaseColor::Red));
        }
        select.add_item(
            StyledString::styled(e.path().file_name().unwrap().to_string_lossy(), style),
            e,
        );
    }
}

fn submit_search(s: &mut Cursive, text: &str) {
    let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name("curr").unwrap();
    let curr_select = curr.get_inner_mut();
    let query = text.replace("search: ", "").to_ascii_lowercase();
    let result = curr_select.iter().find(|x| {
        x.0.to_ascii_lowercase().eq(&query) || x.0.to_ascii_lowercase().starts_with(&query)
    });
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
    populate_select(
        prev_select,
        env::current_dir().unwrap().parent().unwrap(),
        show_hidden,
    );
    update_prev(prev_select);
    prev.scroll_to_important_area();

    let curr_selection = curr.get_inner().selection().unwrap();
    update_next(s, &curr_selection);
}

fn main() {
    let mut siv = cursive::default();
    // let mut siv = cursive_extras::buffered_backend_root();

    let mut theme = Theme::terminal_default();
    theme.palette[PaletteColor::Highlight] = Color::Dark(BaseColor::Red);
    theme.palette[PaletteColor::HighlightInactive] = Color::Dark(BaseColor::Red);
    siv.set_theme(theme);

    let state = State::new();
    siv.set_user_data(state);

    siv.add_fullscreen_layer(Layer::new(vlayout!(
        TextView::new(env::current_dir().unwrap().to_string_lossy()).with_name("path_text"),
        hlayout!(
            ShadowView::new(
                SelectView::<DirEntry>::new()
                    .disabled()
                    .scrollable()
                    .show_scrollbars(false)
                    .with_name("prev")
                    .fixed_width(15)
            )
            .top_padding(false)
            .left_padding(false),
            ShadowView::new(
                SelectView::<DirEntry>::new()
                    .on_select(update_next)
                    .scrollable()
                    .show_scrollbars(false)
                    .with_name("curr")
                    .min_width(30)
            )
            .top_padding(false)
        )
        .with_name("hlayout")
        .full_height(),
        EditView::new()
            .disabled()
            .filler(" ")
            .on_submit(submit_search)
            .with_name("search")
            .fixed_height(1)
    )));

    init(&mut siv);

    siv.focus_name("curr").unwrap();
    siv.add_global_callback('q', |s| {
        let cwd = env::current_dir().unwrap().to_str().unwrap().to_owned();
        fs::write("/tmp/seldir", cwd).unwrap();
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
