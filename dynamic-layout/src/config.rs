use std::{str::FromStr, time::Duration};

use dynisland_core::{
    abi::{gdk, glib, gtk, log, module::ActivityIdentifier},
    graphics::activity_widget::{boxed_activity_mode::ActivityMode, ActivityWidget},
};
use gtk::{prelude::*, EventController, StateFlags};
use serde::{Deserialize, Serialize};

use crate::{
    layout::DynamicLayout, priority_order::WidgetOrderManager, window_position::WindowPosition,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct DynamicLayoutConfig {
    pub(super) window_position: WindowPosition,
    pub(super) auto_minimize_timeout: i32,
    pub(super) max_activities: u16,
    pub(super) max_active: u16,
    pub(super) reorder_on_add: bool,
    pub(super) reorder_on_reload: bool,
    pub(super) activity_order: Vec<String>,
}

impl Default for DynamicLayoutConfig {
    fn default() -> Self {
        Self {
            // orientation_horizontal: true,
            window_position: WindowPosition::default(),
            auto_minimize_timeout: 5000,
            max_activities: 3,
            max_active: 1,
            reorder_on_add: true,
            reorder_on_reload: true,
            activity_order: Vec::new(),
        }
    }
}
#[derive(Debug, Clone)]
pub enum ActivityMatch {
    Activity(ActivityIdentifier),
    Module(String),
    Other,
}
impl FromStr for ActivityMatch {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split = s.split('@').collect::<Vec<&str>>();
        match split.len() {
            1 => Ok(ActivityMatch::Module(split[0].to_string())),
            2 => Ok(ActivityMatch::Activity(ActivityIdentifier::new(
                split[1], split[0],
            ))),
            _ => Err(String::from("invalid match")),
        }
    }
}
impl ActivityMatch {
    pub(super) fn contains(&self, id: &ActivityIdentifier) -> bool {
        match self {
            ActivityMatch::Activity(activity_id) => {
                id.module().eq_ignore_ascii_case(&activity_id.module())
                    && id.activity().eq_ignore_ascii_case(&activity_id.activity())
            }
            ActivityMatch::Module(module) => id.module().eq_ignore_ascii_case(module),
            ActivityMatch::Other => true,
        }
    }
}

impl DynamicLayoutConfig {
    pub(super) fn get_order(&self) -> Vec<ActivityMatch> {
        let mut matches = Vec::new();
        for rule in self.activity_order.iter() {
            if let Ok(rule) = ActivityMatch::from_str(rule) {
                matches.push(rule);
            }
        }
        matches
    }
}

impl<Ord: WidgetOrderManager> DynamicLayout<Ord> {
    pub(crate) fn configure_widget(&self, activity_id: &ActivityIdentifier) {
        let borrow = self.order_manager.borrow().get_widget_map();
        let widget_map = borrow.borrow();
        let widget = widget_map.get(activity_id).unwrap();

        widget.set_valign(self.config.window_position.v_anchor.map_gtk());
        widget.set_halign(self.config.window_position.h_anchor.map_gtk());

        // remove old controllers
        let mut controllers_removed = 0;
        let mut controllers = vec![];
        for controller in widget
            .observe_controllers()
            .iter::<glib::Object>()
            .flatten()
            .flat_map(|c| c.downcast::<EventController>())
        {
            if let Some(name) = controller.name() {
                if name == "press_gesture" || name == "focus_controller" {
                    controllers.push(controller);
                    controllers_removed += 1;
                }
            }
        }
        for controller in controllers.iter() {
            widget.remove_controller(controller);
        }
        // connect deactivate if it's not already connected
        if controllers_removed == 0 {
            let send_deactivate = self.deactivate_widget.0.clone();
            let id = activity_id.clone();
            widget.connect_mode_notify(move |aw| {
                if aw.last_mode() == ActivityMode::Minimal || aw.mode() != ActivityMode::Minimal {
                    return;
                }
                if let Err(err) = send_deactivate.send(id.clone()) {
                    log::error!("error activating widget: {err}");
                }
            });
        }

        let press_gesture = gtk::GestureClick::new();
        press_gesture.set_name(Some("press_gesture"));
        let send_activate = self.activate_widget.0.clone();
        let send_cycle = self.cycle_channel.0.clone();
        // Minimal mode to Compact mode controller
        press_gesture.set_button(gdk::BUTTON_PRIMARY);
        let id = activity_id.clone();
        press_gesture.connect_released(move |gest, _, x, y| {
            let aw = gest.widget().downcast::<ActivityWidget>().unwrap();
            if x < 0.0
                || y < 0.0
                || x > aw.size(gtk::Orientation::Horizontal).into()
                || y > aw.size(gtk::Orientation::Vertical).into()
            {
                // cycle by dragging
                if aw.mode() == ActivityMode::Minimal {
                    if let Err(err) = send_cycle.send(x > 0.0) {
                        log::error!("error activating widget: {err}");
                    }
                }
                return;
            }
            if let ActivityMode::Minimal = aw.mode() {
                if let Err(err) = send_activate.send(id.clone()) {
                    log::error!("error activating widget: {err}");
                    return;
                }
                // aw.set_mode(ActivityMode::Compact);
                gest.set_state(gtk::EventSequenceState::Claimed);
            }
        });
        widget.add_controller(press_gesture);

        // auto minimize (to Compact mode) controller
        let focus_controller = gtk::EventControllerMotion::new();
        focus_controller.set_name(Some("focus_controller"));
        if self.config.auto_minimize_timeout >= 0 {
            let cancel_minimize = self.cancel_minimize.clone();
            let timeout = self.config.auto_minimize_timeout;
            let activity_id = activity_id.clone();
            focus_controller.connect_leave(move |evt| {
                let aw = evt.widget().downcast::<ActivityWidget>().unwrap();
                let mode = aw.mode();
                if matches!(mode, ActivityMode::Minimal | ActivityMode::Compact) {
                    return;
                }
                let id = glib::timeout_add_local_once(
                    Duration::from_millis(timeout.try_into().unwrap()),
                    move || {
                        if !aw.state_flags().contains(StateFlags::PRELIGHT) && aw.mode() == mode {
                            //mouse is not on widget and mode hasn't changed
                            aw.set_mode(ActivityMode::Compact);
                        }
                    },
                );
                let mut cancel_minimize = cancel_minimize.borrow_mut();
                if let Some(source) = cancel_minimize.remove(&activity_id) {
                    if glib::MainContext::default()
                        .find_source_by_id(&source)
                        .is_some()
                    {
                        source.remove();
                    }
                }

                cancel_minimize.insert(activity_id.clone(), id);
            });
            widget.add_controller(focus_controller);
        }
    }

    pub(crate) fn configure_container(&self) {
        let container = if self.order_manager.borrow().get_container().is_none() {
            return;
        } else {
            let cont = self.order_manager.borrow().get_container();
            cont.as_ref().unwrap().clone()
        };
        container.set_spacing(0);
        // if self.config.orientation_horizontal {
        container.set_orientation(gtk::Orientation::Horizontal);
        // } else {
        // container.set_orientation(gtk::Orientation::Vertical);
        // }
        if !self.config.window_position.layer_shell {
            container.set_halign(self.config.window_position.h_anchor.map_gtk());
            container.set_valign(self.config.window_position.v_anchor.map_gtk());
        }
    }
}
