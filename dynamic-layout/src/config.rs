use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    str::FromStr,
    time::Duration,
};

use dynisland_core::{
    abi::{gdk, glib, gtk, log, module::ActivityIdentifier},
    graphics::activity_widget::{boxed_activity_mode::ActivityMode, ActivityWidget},
};
use gtk::{prelude::*, EventController, StateFlags};
use serde::{Deserialize, Serialize};

use crate::{
    layout::DynamicLayout,
    window_position::{DeWindowPosition, WindowPosition},
};

// TODO: cleanup

#[derive(Debug, Serialize, Clone)]
#[serde(default)]
pub struct DynamicLayoutConfigMain {
    pub(crate) window_position: WindowPosition,
    pub(crate) auto_minimize_timeout: i32,
    pub(crate) max_activities: u16,
    pub(crate) max_active: u16,
    pub(crate) reorder_on_add: bool,
    pub(crate) reorder_on_reload: bool,
    pub(crate) windows: HashMap<String, DynamicLayoutConfig>,
}

pub const DEFAULT_AUTO_MINIMIZE_TIMEOUT: i32 = 5000;

impl Default for DynamicLayoutConfigMain {
    fn default() -> Self {
        let mut map = HashMap::new();
        map.insert("".to_string(), DynamicLayoutConfig::default());
        Self {
            window_position: WindowPosition::default(),
            auto_minimize_timeout: DEFAULT_AUTO_MINIMIZE_TIMEOUT,
            max_activities: 3,
            max_active: 1,
            reorder_on_add: true,
            reorder_on_reload: true,
            windows: map,
        }
    }
}

impl DynamicLayoutConfigMain {
    pub fn default_conf(&self) -> DynamicLayoutConfig {
        DynamicLayoutConfig {
            window_position: self.window_position.clone(),
            auto_minimize_timeout: self.auto_minimize_timeout,
            max_activities: self.max_activities,
            max_active: self.max_active,
            reorder_on_add: self.reorder_on_add,
            reorder_on_reload: self.reorder_on_reload,
            activity_order: Vec::new(),
        }
    }
    pub fn get_for_window(&self, window: &str) -> DynamicLayoutConfig {
        match self.windows.get(window) {
            Some(conf) => conf.clone(),
            None => self.default_conf(),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(default)]
pub struct DynamicLayoutConfig {
    #[serde(skip_serializing)]
    pub(crate) window_position: WindowPosition,
    #[serde(skip_serializing)]
    pub(crate) auto_minimize_timeout: i32,
    #[serde(skip_serializing)]
    pub(crate) max_activities: u16,
    #[serde(skip_serializing)]
    pub(crate) max_active: u16,
    #[serde(skip_serializing)]
    pub(crate) reorder_on_add: bool,
    #[serde(skip_serializing)]
    pub(crate) reorder_on_reload: bool,
    pub(crate) activity_order: Vec<ActivityMatch>,
}
impl Default for DynamicLayoutConfig {
    fn default() -> Self {
        Self {
            window_position: WindowPosition::default(),
            auto_minimize_timeout: DEFAULT_AUTO_MINIMIZE_TIMEOUT,
            max_activities: 3,
            max_active: 1,
            reorder_on_add: true,
            reorder_on_reload: true,
            activity_order: Vec::new(),
        }
    }
}
#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DeDynamicLayoutConfigMain {
    window_position: WindowPosition,
    auto_minimize_timeout: i32,
    max_activities: u16,
    max_active: u16,
    reorder_on_add: bool,
    reorder_on_reload: bool,
    windows: HashMap<String, DeDynamicLayoutConfig>,
}

impl Default for DeDynamicLayoutConfigMain {
    fn default() -> Self {
        Self {
            window_position: WindowPosition::default(),
            auto_minimize_timeout: DEFAULT_AUTO_MINIMIZE_TIMEOUT,
            max_activities: 3,
            max_active: 1,
            reorder_on_add: true,
            reorder_on_reload: true,
            windows: HashMap::new(),
        }
    }
}

impl DeDynamicLayoutConfigMain {
    pub fn into_main_config(self) -> DynamicLayoutConfigMain {
        let mut windows = HashMap::new();
        for (name, opt_config) in self.windows {
            let window_pos = match opt_config.window_position {
                Some(opt_window_pos) => WindowPosition {
                    layer: opt_window_pos
                        .layer
                        .unwrap_or(self.window_position.layer.clone()),
                    h_anchor: opt_window_pos
                        .h_anchor
                        .unwrap_or(self.window_position.h_anchor.clone()),
                    v_anchor: opt_window_pos
                        .v_anchor
                        .unwrap_or(self.window_position.v_anchor.clone()),
                    margin_x: opt_window_pos
                        .margin_x
                        .unwrap_or(self.window_position.margin_x),
                    margin_y: opt_window_pos
                        .margin_y
                        .unwrap_or(self.window_position.margin_y),
                    exclusive_zone: opt_window_pos
                        .exclusive_zone
                        .unwrap_or(self.window_position.exclusive_zone),
                    monitor: opt_window_pos
                        .monitor
                        .unwrap_or(self.window_position.monitor.clone()),
                    layer_shell: opt_window_pos
                        .layer_shell
                        .unwrap_or(self.window_position.layer_shell),
                },
                None => self.window_position.clone(),
            };
            let conf = DynamicLayoutConfig {
                window_position: window_pos,
                auto_minimize_timeout: opt_config
                    .auto_minimize_timeout
                    .unwrap_or(self.auto_minimize_timeout),
                max_activities: opt_config.max_activities.unwrap_or(self.max_activities),
                max_active: opt_config.max_active.unwrap_or(self.max_active),
                reorder_on_add: opt_config.reorder_on_add.unwrap_or(self.reorder_on_add),
                reorder_on_reload: opt_config
                    .reorder_on_reload
                    .unwrap_or(self.reorder_on_reload),
                activity_order: DeDynamicLayoutConfig::get_order(opt_config.activity_order),
            };
            windows.insert(name, conf);
        }
        let mut main_conf = DynamicLayoutConfigMain {
            window_position: self.window_position,
            auto_minimize_timeout: self.auto_minimize_timeout,
            max_activities: self.max_activities,
            max_active: self.max_active,
            reorder_on_add: self.reorder_on_add,
            reorder_on_reload: self.reorder_on_reload,
            windows,
        };
        if !main_conf.windows.contains_key("") {
            let default = main_conf.default_conf();
            main_conf.windows.insert("".to_string(), default);
        }
        main_conf
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct DeDynamicLayoutConfig {
    window_position: Option<DeWindowPosition>,
    auto_minimize_timeout: Option<i32>,
    max_activities: Option<u16>,
    max_active: Option<u16>,
    reorder_on_add: Option<bool>,
    reorder_on_reload: Option<bool>,
    activity_order: Option<Vec<String>>,
}

impl DeDynamicLayoutConfig {
    pub(super) fn get_order(order: Option<Vec<String>>) -> Vec<ActivityMatch> {
        let mut matches = Vec::new();
        if let Some(order) = order {
            for rule in order.iter() {
                if let Ok(rule) = ActivityMatch::from_str(rule) {
                    matches.push(rule);
                }
            }
        }
        matches
    }
}

#[derive(Debug, Clone)]
pub enum ActivityMatch {
    Activity(ActivityIdentifier),
    Module(String),
    Other,
}
impl Display for ActivityMatch {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivityMatch::Activity(id) => write!(f, "{}@{}", id.activity(), id.module()),
            ActivityMatch::Module(module) => write!(f, "{}", module),
            ActivityMatch::Other => write!(f, "*"),
        }
    }
}
impl Serialize for ActivityMatch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let disp = self.to_string();
        serializer.serialize_str(&disp)
    }
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
    pub(crate) fn contains(&self, id: &ActivityIdentifier) -> bool {
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

impl DynamicLayout {
    pub(crate) fn configure_widget(&self, activity_id: &ActivityIdentifier) {
        let (widget, window_name) =
            Self::find_widget(&self.order_managers.borrow(), activity_id).unwrap();

        widget.set_valign(
            self.config
                .get_for_window(&window_name)
                .window_position
                .v_anchor
                .map_gtk(),
        );
        widget.set_halign(
            self.config
                .get_for_window(&window_name)
                .window_position
                .h_anchor
                .map_gtk(),
        );

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
        let window_name1 = window_name.clone();
        press_gesture.connect_released(move |gest, _, x, y| {
            let aw = gest.widget().downcast::<ActivityWidget>().unwrap();
            if x < 0.0
                || y < 0.0
                || x > aw.size(gtk::Orientation::Horizontal).into()
                || y > aw.size(gtk::Orientation::Vertical).into()
            {
                // cycle by dragging
                if aw.mode() == ActivityMode::Minimal {
                    if let Err(err) = send_cycle.send((window_name1.clone(), x > 0.0)) {
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
        if self
            .config
            .get_for_window(&window_name)
            .auto_minimize_timeout
            >= 0
        {
            let cancel_minimize = self.cancel_minimize.clone();
            let timeout = self
                .config
                .get_for_window(&window_name)
                .auto_minimize_timeout;
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

    pub(crate) fn configure_container(&self, window_name: &str) {
        let container = self
            .order_managers
            .borrow()
            .get(window_name)
            .unwrap()
            .borrow()
            .get_container()
            .clone();
        container.set_spacing(0);
        // if self.config.orientation_horizontal {
        container.set_orientation(gtk::Orientation::Horizontal);
        // } else {
        // container.set_orientation(gtk::Orientation::Vertical);
        // }
        let config = self.config.get_for_window(&window_name);
        if !config.window_position.layer_shell {
            container.set_halign(config.window_position.h_anchor.map_gtk());
            container.set_valign(config.window_position.v_anchor.map_gtk());
        }
    }
}
