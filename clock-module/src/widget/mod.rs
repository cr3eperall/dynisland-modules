use clock::Clock;
use compact::Compact;
use dynisland_core::{
    dynamic_activity::DynamicActivity,
    dynamic_property::PropertyUpdate,
    graphics::activity_widget::{boxed_activity_mode::ActivityMode, ActivityWidget},
};
use gtk::{prelude::*, GestureClick};

pub mod clock;
pub mod compact;

pub fn get_activity(
    prop_send: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
    module: &str,
    name: &str,
) -> DynamicActivity {
    let mut dynamic_act = DynamicActivity::new(prop_send, module, name);

    let activity_widget = dynamic_act.get_activity_widget();

    let minimal = Clock::new(&mut dynamic_act);
    minimal.set_width_request(30);
    minimal.set_height_request(30);
    minimal.set_valign(gtk::Align::Center);
    minimal.set_halign(gtk::Align::Center);

    let compact = Compact::new(&mut dynamic_act);

    activity_widget.set_minimal_mode_widget(minimal);
    activity_widget.set_compact_mode_widget(compact);

    register_mode_gestures(activity_widget);

    dynamic_act
}

fn register_mode_gestures(activity_widget: ActivityWidget) {
    // let primary_gesture = gtk::GestureClick::new();
    // primary_gesture.set_button(gdk::BUTTON_PRIMARY);

    // primary_gesture.connect_released(move |gest, _, x, y| {
    //     let aw = gest.widget().downcast::<ActivityWidget>().unwrap();
    //     if x < 0.0
    //         || y < 0.0
    //         || x > aw.size(gtk::Orientation::Horizontal).into()
    //         || y > aw.size(gtk::Orientation::Vertical).into()
    //     {
    //         return;
    //     }
    //     match aw.mode() {
    //         ActivityMode::Compact => {
    //             aw.set_mode(ActivityMode::Expanded);
    //         }
    //         ActivityMode::Expanded => {
    //             aw.set_mode(ActivityMode::Overlay);
    //         }
    //         _ => {}
    //     }
    // });

    // activity_widget.add_controller(primary_gesture);

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
