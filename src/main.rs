use bit::BitIndex;
use chrono::prelude::*;
use clap::{arg, command};
use cursive::align::HAlign;
use cursive::event::{Event, Key};
use cursive::theme::{Color, ColorStyle, ColorType, PaletteColor, Theme};
use cursive::utils::markup::StyledString;
use cursive::view::{Nameable, Resizable, Scrollable, Selector};
use cursive::views::{
    EditView, Layer, LayerPosition, LinearLayout, ScrollView, SelectView, ShadowView, StackView,
    TextView, ViewRef,
};
use cursive::{Cursive, View};
use cursive_extras::{hlayout, vlayout};
use std::cmp::Ordering;
use std::fs::DirEntry;
use std::os::unix::prelude::PermissionsExt;
use std::path::Path;
use std::{env, fs};

const CURR_NAME: &str = "curr";
const PREV_NAME: &str = "prev";
const NEXT_NAME: &str = "next";
const SEARCH_NAME: &str = "search";
const HLAYOUT_NAME: &str = "hlayout";
const VLAYOUT_NAME: &str = "vlayout";
const PATH_TEXT_NAME: &str = "path_text";
const ID_TEXT_NAME: &str = "id_text";
const PERMISSIONS_TEXT_NAME: &str = "permissions";
const LAST_MOD_TEXT_NAME: &str = "last_modified";
const STACK_VIEW_NAME: &str = "stack_view";
const STATS_NAME: &str = "stats";
const EDIT_VIEW_NAME: &str = "edit_view";

struct State {
    show_hidden: bool,
    goto_mode: bool,
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

fn get_symbolic_permissions(permissions: u32) -> String {
    let mut symbolic = String::new();
    symbolic += if permissions.bit(14) { "d" } else { "-" };
    for i in (2..11).step_by(3).rev() {
        symbolic += if permissions.bit(i) { "r" } else { "-" };
        symbolic += if permissions.bit(i - 1) { "w" } else { "-" };
        symbolic += if permissions.bit(i - 2) { "x" } else { "-" };
    }
    symbolic
}

fn update_next(s: &mut Cursive, item: &DirEntry) {
    let mut hlayout: ViewRef<LinearLayout> = s.find_name(HLAYOUT_NAME).unwrap();
    let mut path_text: ViewRef<TextView> = s.find_name(PATH_TEXT_NAME).unwrap();
    let mut permissions_text: ViewRef<TextView> = s.find_name(PERMISSIONS_TEXT_NAME).unwrap();
    let mut last_modified_text: ViewRef<TextView> = s.find_name(LAST_MOD_TEXT_NAME).unwrap();
    let curr_dir = env::current_dir().unwrap();
    let curr_dir_str = curr_dir.to_str().unwrap();

    path_text.set_content(format!(
        "{}/{}",
        if curr_dir_str.eq("/") {
            "".to_owned()
        } else {
            curr_dir_str.to_owned()
        },
        item.file_name().to_string_lossy()
    ));

    let metadata = item.metadata().unwrap();
    let last_modified = metadata.modified().unwrap();
    let date_time: DateTime<Utc> = last_modified.into();
    permissions_text.set_content(get_symbolic_permissions(metadata.permissions().mode()));
    last_modified_text.set_content(format!(" {}", date_time.format("%d-%m-%Y %H:%M")));

    hlayout.remove_child(2);

    if item.path().is_dir() {
        let mut next_select = SelectView::<DirEntry>::new()
            .disabled()
            .with_inactive_highlight(false);

        populate_select(s, &mut next_select, &item.path());
        hlayout.add_child(
            ShadowView::new(next_select.scrollable().show_scrollbars(false))
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

fn search(s: &mut Cursive, text: &str) {
    let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(CURR_NAME).unwrap();
    let curr_select = curr.get_inner_mut();
    let query = text.to_ascii_lowercase();
    let result = curr_select.iter().find(|x| {
        x.0.to_ascii_lowercase().eq(&query) || x.0.to_ascii_lowercase().starts_with(&query)
    });
    if let Some(item) = result {
        let item_id = curr_select.iter().position(|x| x.0.eq(item.0)).unwrap();
        let cb = curr_select.set_selection(item_id);
        cb(s);
        curr.scroll_to_important_area();
        update_id_text(s, curr.get_inner());
    }
}

fn update_id_text(s: &mut Cursive, curr_select: &SelectView<DirEntry>) {
    let mut num_text: ViewRef<TextView> = s.find_name(ID_TEXT_NAME).unwrap();
    if let Some(id) = curr_select.selected_id() {
        num_text.set_content(format!("{}/{}", id + 1, curr_select.len()));
    }
}

fn change_dir(s: &mut Cursive, is_enter: bool) {
    update_curr(s, is_enter);
    update_prev(s);
    let curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(CURR_NAME).unwrap();
    update_id_text(s, curr.get_inner());
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
    update_id_text(s, curr_select);
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
    println!("{path}");
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

    let state = State {
        show_hidden: false,
        goto_mode: false,
    };
    siv.set_user_data(state);

    siv.add_fullscreen_layer(Layer::new(
        vlayout!(
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
            StackView::new()
                .fullscreen_layer(
                    hlayout!(
                        TextView::new("search: "),
                        EditView::new()
                            .filler(" ")
                            .on_edit(|s, text, _| search(s, text))
                            .with_name(EDIT_VIEW_NAME)
                            .full_width()
                            .fixed_height(1)
                    )
                    .with_name(SEARCH_NAME),
                )
                .fullscreen_layer(
                    hlayout!(
                        TextView::new("")
                            .style(ColorStyle::front(accent_color))
                            .with_name(PERMISSIONS_TEXT_NAME),
                        TextView::new("").with_name(LAST_MOD_TEXT_NAME).full_width(),
                        TextView::new("")
                            .h_align(HAlign::Right)
                            .with_name(ID_TEXT_NAME)
                            .full_width()
                    )
                    .with_name(STATS_NAME)
                )
                .with_name(STACK_VIEW_NAME)
                .fixed_height(1)
                .full_width()
        )
        .with_name(VLAYOUT_NAME)
        .full_width(),
    ));

    init(&mut siv);

    siv.focus_name(SEARCH_NAME).unwrap();

    siv.add_global_callback('q', handle_exit);
    siv.add_global_callback(Key::Enter, |s| {
        let mut stack: ViewRef<StackView> = s.find_name(STACK_VIEW_NAME).unwrap();
        if let Some(LayerPosition::FromBack(1)) = stack.find_layer_from_name(SEARCH_NAME) {
            stack.move_to_back(LayerPosition::FromFront(0));
        } else {
            handle_exit(s);
        }
    });
    siv.add_global_callback('j', |s| {
        let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(CURR_NAME).unwrap();
        let cb = curr.get_inner_mut().select_down(1);
        cb(s);
        curr.scroll_to_important_area();
        update_id_text(s, curr.get_inner());
    });
    siv.add_global_callback('k', |s| {
        let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> = s.find_name(CURR_NAME).unwrap();
        let cb = curr.get_inner_mut().select_up(1);
        cb(s);
        curr.scroll_to_important_area();
        update_id_text(s, curr.get_inner());
    });
    siv.add_global_callback('l', |s| change_dir(s, true));
    siv.add_global_callback(Key::Right, |s| change_dir(s, true));
    siv.add_global_callback('h', |s| change_dir(s, false));
    siv.add_global_callback(Key::Left, |s| change_dir(s, false));
    siv.add_global_callback('e', |s| {
        let goto_mode = s.user_data::<State>().unwrap().goto_mode;
        if goto_mode {
            let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> =
                s.find_name(CURR_NAME).unwrap();
            let curr_select = curr.get_inner_mut();
            let cb = curr_select.set_selection(curr_select.len() - 1);
            cb(s);
            curr.scroll_to_important_area();
            s.user_data::<State>().unwrap().goto_mode = false;
        }
    });
    siv.add_global_callback('g', |s| {
        let goto_mode = s.user_data::<State>().unwrap().goto_mode;
        if goto_mode {
            let mut curr: ViewRef<ScrollView<SelectView<DirEntry>>> =
                s.find_name(CURR_NAME).unwrap();
            let cb = curr.get_inner_mut().set_selection(0);
            cb(s);
            curr.scroll_to_important_area();
            s.user_data::<State>().unwrap().goto_mode = false;
        } else {
            s.user_data::<State>().unwrap().goto_mode = true;
        }
    });
    siv.add_global_callback(Event::CtrlChar('h'), |s| {
        s.user_data::<State>().unwrap().show_hidden ^= true;
        // TODO fix selection not the same
        init(s);
    });
    siv.add_global_callback('/', |s| {
        let mut stack: ViewRef<StackView> = s.find_name(STACK_VIEW_NAME).unwrap();
        stack.move_to_front(LayerPosition::FromFront(1));
    });
    siv.add_global_callback(Event::Key(Key::Esc), |s| {
        let mut stack: ViewRef<StackView> = s.find_name(STACK_VIEW_NAME).unwrap();
        if let Some(LayerPosition::FromBack(1)) = stack.find_layer_from_name(SEARCH_NAME) {
            stack.move_to_back(LayerPosition::FromFront(0));
        } else {
            s.quit();
        }
    });

    siv.run();
}

#[cfg(test)]
mod tests {
    use crate::get_symbolic_permissions;

    #[test]
    fn test_permission_to_str() {
        assert_eq!("-rw-r--r--", get_symbolic_permissions(0o644));
        assert_eq!("----------", get_symbolic_permissions(0o000)); // no permissions
        assert_eq!("-rwx------", get_symbolic_permissions(0o700)); // read, write, & execute only for owner
        assert_eq!("-rwxrwx---", get_symbolic_permissions(0o770)); // read, write, & execute for owner and group
        assert_eq!("-rwxrwxrwx", get_symbolic_permissions(0o777)); // read, write, & execute for owner, group and others
        assert_eq!("---x--x--x", get_symbolic_permissions(0o111)); // execute
        assert_eq!("--w--w--w-", get_symbolic_permissions(0o222)); // write
        assert_eq!("--wx-wx-wx", get_symbolic_permissions(0o333)); // write & execute
        assert_eq!("-r--r--r--", get_symbolic_permissions(0o444)); // read
        assert_eq!("-r-xr-xr-x", get_symbolic_permissions(0o555)); // read & execute
        assert_eq!("-rw-rw-rw-", get_symbolic_permissions(0o666)); // read & write
        assert_eq!("-rwxr-----", get_symbolic_permissions(0o740)); // owner can read, write, & execute; group can only read; others have no permissions
    }
}
