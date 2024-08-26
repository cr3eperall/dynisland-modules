mod compact;
mod expanded;
mod minimal;
mod overlay;

use compact::Compact;
use dynisland_core::{
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
) -> DynamicActivity {
    let mut activity = DynamicActivity::new(prop_send, module, name);

    //create activity widget
    let activity_widget = activity.get_activity_widget();

    //get widgets
    let minimal = Minimal::new(&mut activity);
    let compact = Compact::new(&mut activity);
    let expanded = Expanded::new();
    let overlay = Overlay::new();

    //load widgets in the activity widget
    activity_widget.set_minimal_mode_widget(minimal.clone());
    activity_widget.set_compact_mode_widget(compact.clone());
    activity_widget.set_expanded_mode_widget(expanded.clone());
    activity_widget.set_overlay_mode_widget(overlay.clone());

    register_mode_gestures(activity_widget);

    activity
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
