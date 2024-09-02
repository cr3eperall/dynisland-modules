mod compact;
mod expanded;
mod minimal;
mod overlay;
use compact::Compact;
use dynisland_core::{
    abi::{gdk, gtk},
    dynamic_activity::DynamicActivity,
    dynamic_property::PropertyUpdate,
    graphics::activity_widget::{boxed_activity_mode::ActivityMode, ActivityWidget},
};
use expanded::Expanded;
use gtk::{prelude::*, GestureClick};
use minimal::Minimal;
use overlay::Overlay;

pub fn get_activity(
    prop_send: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
    module: &str,
    name: &str,
    window: &str,
    idx: usize,
) -> DynamicActivity {
    let mut dynamic_act = DynamicActivity::new_with_metadata(
        prop_send,
        module,
        &(name.to_string() + "-" + &idx.to_string()),
        Some(window),
        Some(&("instance=".to_string() + &idx.to_string())),
    );
    //create activity widget
    let activity_widget = dynamic_act.get_activity_widget();
    activity_widget.add_css_class(name);

    //get widgets
    let minimal = Minimal::new(&mut dynamic_act);
    let compact = Compact::new(&mut dynamic_act);
    let expanded = Expanded::new();
    let overlay = Overlay::new();

    //load widgets in the activity widget
    activity_widget.set_minimal_mode_widget(minimal.clone());
    activity_widget.set_compact_mode_widget(compact.clone());
    activity_widget.set_expanded_mode_widget(expanded.clone());
    activity_widget.set_overlay_mode_widget(overlay.clone());

    register_mode_gestures(activity_widget);

    dynamic_act
}

fn register_mode_gestures(activity_widget: ActivityWidget) {
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
}
