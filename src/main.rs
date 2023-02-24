use cursive::event::EventResult;
use cursive::theme::{Theme, BorderStyle};
use cursive::{traits::*, Cursive};
use cursive::view::Selector;
use cursive::views::{Dialog, OnEventView, SelectView, TextView, LinearLayout, Layer, TextContent, NamedView, ViewRef};
use std::fs;

fn main() {

    let mut layer = Layer::new(LinearLayout::horizontal().with_name("splits"));
    let mut select_parent = SelectView::<String>::new().with_name("parent");
    let mut select_current = SelectView::<String>::new().with_name("current");

    select_current.get_mut().set_on_submit(|s: &mut Cursive, item: &String| {
        let mut view: ViewRef<LinearLayout> = s.find_name("splits").unwrap();
        view.swap_children(0, 1);
    });
    
    let paths_current = fs::read_dir("./").unwrap();
    let paths_parent = fs::read_dir("../").unwrap();

    for path in paths_current {
        select_current.get_mut().add_item_str(path.unwrap().path().to_str().unwrap().replace("./", ""));
    }

    for path in paths_parent {
        select_parent.get_mut().add_item_str(path.unwrap().path().to_str().unwrap().replace("../", ""));
    }

    let select_current = OnEventView::new(select_current)
        .on_pre_event_inner('k', |s, _| {
            let cb = s.get_mut().select_up(1);
            Some(EventResult::Consumed(Some(cb)))
        })
        .on_pre_event_inner('j', |s, _| {
            let cb = s.get_mut().select_down(1);
            Some(EventResult::Consumed(Some(cb)))
        });

    let mut siv = cursive::default();
    let mut theme = Theme::terminal_default();
    theme.borders = BorderStyle::None;

    siv.set_theme(theme);


    layer.get_inner_mut().get_mut().add_child(Dialog::around(select_parent).title("parent").full_height().full_width());
    layer.get_inner_mut().get_mut().add_child(Dialog::around(select_current).title("current").full_height().full_width());
    layer.get_inner_mut().get_mut().add_child(Dialog::around(TextView::new_with_content(TextContent::new("content"))).title("preview").full_height().full_width());

    siv.add_layer(layer);
    siv.focus_name("current").expect("current not found");


    siv.run();
}

fn on_select(layer: &mut Layer<LinearLayout>) {
    layer.get_inner_mut().swap_children(0, 1);
}