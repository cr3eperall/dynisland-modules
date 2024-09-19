pub mod compact;
pub mod expanded;
pub mod minimal;
pub mod status_notifier_widgets;

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

pub fn get_activity(
    prop_send: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
    module: &str,
    name: &str,
    window: &str,
    idx: usize,
) -> DynamicActivity {
    // create the dynamic activity
    let mut dynamic_act = DynamicActivity::new_with_metadata(
        prop_send,
        module,
        &(name.to_string() + "-" + &idx.to_string()),
        Some(window),
        vec![("instance".to_string(), idx.to_string())],
    );

    // get the activity widget
    let activity_widget = dynamic_act.get_activity_widget();
    activity_widget.add_css_class(name);

    //create the widgets for each mode
    let minimal = Minimal::new(&mut dynamic_act);

    let compact = Compact::new();

    let expanded = Expanded::new();

    // load widgets into the activity widget
    activity_widget.set_minimal_mode_widget(minimal);
    activity_widget.set_compact_mode_widget(compact);
    activity_widget.set_expanded_mode_widget(expanded);

    // register the gestures for changing mode
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
            // Not used because the transition from minimal to compact is done by the layout manager
            // but this is the way to do it for the other modes
            // ActivityMode::Minimal => {
            //     aw.set_mode(ActivityMode::Compact);
            // }
            _ => {}
        }
    });

    activity_widget.add_controller(primary_gesture);

    let secondary_gesture = GestureClick::new();
    secondary_gesture.set_button(gdk::BUTTON_SECONDARY);
    secondary_gesture.set_propagation_phase(gtk::PropagationPhase::Bubble);
    secondary_gesture.connect_released(move |gest, _, x, y| {
        let aw = gest.widget().downcast::<ActivityWidget>().unwrap();
        if x < 0.0
            || y < 0.0
            || x > aw.size(gtk::Orientation::Horizontal).into()
            || y > aw.size(gtk::Orientation::Vertical).into()
        {
            gest.set_state(gtk::EventSequenceState::None);
            return;
        }
        match aw.mode() {
            // this should be kept even if there is no compact mode
            ActivityMode::Compact => {
                aw.set_mode(ActivityMode::Minimal);
                gest.set_state(gtk::EventSequenceState::Claimed);
            }
            ActivityMode::Expanded => {
                aw.set_mode(ActivityMode::Compact);
                gest.set_state(gtk::EventSequenceState::Claimed);
            }
            _ => {
                gest.set_state(gtk::EventSequenceState::None);
            }
        }
    });
    activity_widget.add_controller(secondary_gesture);
}
