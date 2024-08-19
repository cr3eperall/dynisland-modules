use dynisland_core::{
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
    dynamic_property::PropertyUpdate,
    graphics::{
        activity_widget::{boxed_activity_mode::ActivityMode, ActivityWidget},
        widgets::{rolling_char::RollingChar, scrolling_label::ScrollingLabel},
    },
};

use gtk::{prelude::*, GestureClick, Label};

pub fn get_activity(
    prop_send: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
    module: &str,
    name: &str,
) -> DynamicActivity {
    let mut activity = DynamicActivity::new(prop_send, module, name);

    //create activity widget
    let activity_widget = activity.get_activity_widget();
    //get widgets
    let minimal = get_minimal();
    let compact = get_compact();
    let expanded = get_expanded();
    let overlay = get_overlay();

    //load widgets in the activity widget
    activity_widget.set_minimal_mode_widget(&minimal);
    activity_widget.set_compact_mode_widget(&compact);
    activity_widget.set_expanded_mode_widget(&expanded);
    activity_widget.set_overlay_mode_widget(&overlay);

    activity
        .add_dynamic_property("comp-label", "compact".to_string())
        .unwrap();
    activity
        .add_dynamic_property("scrolling-label-text", "Hello, World".to_string())
        .unwrap();
    activity.add_dynamic_property("rolling-char", '0').unwrap();

    let minimal_cl = minimal.clone();
    activity
        .subscribe_to_property("scrolling-label-text", move |new_value| {
            let real_value = cast_dyn_any!(new_value, String).unwrap();
            log::trace!("text changed:{real_value}");
            minimal_cl
                .downcast_ref::<gtk::Box>()
                .unwrap()
                .first_child()
                .unwrap()
                .downcast::<ScrollingLabel>()
                .unwrap()
                .set_text(real_value.as_str());
        })
        .unwrap();

    let primary_gesture = gtk::GestureClick::new();
    primary_gesture.set_button(gdk::BUTTON_PRIMARY);

    primary_gesture.connect_released(move |gest, _, x, y| {
        let aw = gest.widget().downcast::<ActivityWidget>().unwrap();
        if x < 0.0
            || y < 0.0
            || x > aw.size(gtk::Orientation::Horizontal).into()
            || y > aw.size(gtk::Orientation::Vertical).into()
        {
            return;
        }
        match aw.mode() {
            ActivityMode::Compact => {
                aw.set_mode(ActivityMode::Expanded);
            }
            ActivityMode::Expanded => {
                aw.set_mode(ActivityMode::Overlay);
            }
            _ => {}
        }
    });

    activity_widget.add_controller(primary_gesture);

    let secondary_gesture = GestureClick::new();
    secondary_gesture.set_button(gdk::BUTTON_SECONDARY);
    secondary_gesture.connect_released(move |gest, _, x, y| {
        let aw = gest.widget().downcast::<ActivityWidget>().unwrap();
        if x < 0.0
            || y < 0.0
            || x > aw.size(gtk::Orientation::Horizontal).into()
            || y > aw.size(gtk::Orientation::Vertical).into()
        {
            return;
        }
        match aw.mode() {
            ActivityMode::Compact => {
                aw.set_mode(ActivityMode::Minimal);
            }
            ActivityMode::Expanded => {
                aw.set_mode(ActivityMode::Compact);
            }
            ActivityMode::Overlay => {
                aw.set_mode(ActivityMode::Expanded);
            }
            _ => {}
        }
    });
    activity_widget.add_controller(secondary_gesture);

    let c1 = compact.clone();
    activity
        .subscribe_to_property("rolling-char", move |new_value| {
            let real_value = cast_dyn_any!(new_value, char).unwrap();
            let first_child = c1
                .downcast_ref::<gtk::Box>()
                .unwrap()
                .first_child()
                .unwrap();

            let rolling_char_1 = first_child
                .next_sibling()
                .unwrap()
                .downcast::<RollingChar>()
                .unwrap();
            rolling_char_1.set_current_char(real_value);

            let rolling_char_2 = rolling_char_1
                .next_sibling()
                .unwrap()
                .downcast::<RollingChar>()
                .unwrap();
            rolling_char_2.set_current_char(real_value);
        })
        .unwrap();

    //set label when updated
    activity
        .subscribe_to_property("comp-label", move |new_value| {
            let real_value = cast_dyn_any!(new_value, String).unwrap();
            compact
                .downcast_ref::<gtk::Box>()
                .unwrap()
                .first_child()
                .unwrap()
                .downcast::<gtk::Label>()
                .unwrap()
                .set_label(real_value);
        })
        .unwrap();
    activity
}

fn get_minimal() -> gtk::Widget {
    let minimal = gtk::Box::builder()
        .height_request(40)
        .width_request(50)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .overflow(gtk::Overflow::Hidden)
        .homogeneous(false)
        .build();

    let scroll_label = ScrollingLabel::new();
    scroll_label.set_text("Scrolling Label");
    scroll_label.set_hexpand(false);
    scroll_label.set_vexpand(false);
    scroll_label.set_valign(gtk::Align::Center);
    scroll_label.set_halign(gtk::Align::Start);
    scroll_label.set_width_request(40);
    scroll_label.set_height_request(40);

    minimal.append(&scroll_label);
    minimal.upcast()
}

fn get_compact() -> gtk::Widget {
    let compact = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .height_request(40)
        .width_request(220)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();

    compact.append(
        &Label::builder()
            .label("Compact")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .build(),
    );

    let rn1 = RollingChar::new(None);
    rn1.set_valign(gtk::Align::Center);
    rn1.set_halign(gtk::Align::Center);
    compact.append(&rn1);

    let rn2 = RollingChar::new(None);
    rn2.set_valign(gtk::Align::Center);
    rn2.set_halign(gtk::Align::Center);
    compact.append(&rn2);

    compact.upcast()
}

fn get_expanded() -> gtk::Widget {
    let expanded = gtk::Box::builder()
        .height_request(400)
        .width_request(500)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();

    expanded.append(
        &gtk::Label::builder()
            .label("Expanded label,\n Hello World")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build(),
    );
    expanded.upcast()
}

fn get_overlay() -> gtk::Widget {
    let expanded = gtk::Box::builder()
        .height_request(1080)
        .width_request(1920)
        .valign(gtk::Align::Center)
        .halign(gtk::Align::Center)
        .vexpand(false)
        .hexpand(false)
        .build();

    expanded.append(
        &gtk::Label::builder()
            .label("Overlay label,\n Hello World \n Hello World")
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .build(),
    );
    expanded.upcast()
}
