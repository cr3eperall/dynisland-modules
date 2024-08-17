use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};

use abi_stable::{
    sabi_extern_fn,
    sabi_trait::TD_CanDowncast,
    std_types::{
        RBoxError, ROption,
        RResult::{self, RErr, ROk},
        RString, RVec,
    },
};
use anyhow::Context;
use dynisland_abi::{
    layout::{LayoutManagerType, SabiLayoutManager, SabiLayoutManager_TO},
    module::ActivityIdentifier,
    SabiApplication, SabiWidget,
};
use dynisland_core::graphics::activity_widget::{
    boxed_activity_mode::ActivityMode, ActivityWidget,
};
use env_logger::Env;
use gdk::prelude::*;
use glib::Cast;
use gtk::{prelude::*, EventController, StateFlags};
use log::Level;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{
    priority_order::{cycle_order::CycleOrder, ui_update::UiUpdate, WidgetOrderManager},
    window_position::WindowPosition,
};

pub struct DynamicLayout<Ord: WidgetOrderManager> {
    app: gtk::Application,
    widget_map: Rc<RefCell<HashMap<ActivityIdentifier, ActivityWidget>>>,
    container: Option<gtk::Box>,
    priority: Rc<RefCell<Ord>>,
    activate_widget: (
        UnboundedSender<ActivityIdentifier>,
        Option<UnboundedReceiver<ActivityIdentifier>>,
    ),
    deactivate_widget: (
        UnboundedSender<ActivityIdentifier>,
        Option<UnboundedReceiver<ActivityIdentifier>>,
    ),
    cycle_channel: (UnboundedSender<bool>, Option<UnboundedReceiver<bool>>),
    config: DynamicLayoutConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct DynamicLayoutConfig {
    // TODO add layer
    pub(super) window_position: WindowPosition,
    // orientation_horizontal: bool,
    pub(super) auto_minimize_timeout: i32,
    pub(super) max_activities: u16,
    pub(super) max_active: u16,
}
impl Default for DynamicLayoutConfig {
    fn default() -> Self {
        Self {
            // orientation_horizontal: true,
            window_position: WindowPosition::default(),
            auto_minimize_timeout: 5000,
            max_activities: 3,
            max_active: 1,
        }
    }
}

#[sabi_extern_fn]
pub fn new(app: SabiApplication) -> RResult<LayoutManagerType, RBoxError> {
    env_logger::Builder::from_env(Env::default().default_filter_or(Level::Warn.as_str())).init();

    let app = app.try_into().unwrap();
    let channel = tokio::sync::mpsc::unbounded_channel();
    let channel1 = tokio::sync::mpsc::unbounded_channel();
    let channel2 = tokio::sync::mpsc::unbounded_channel();
    let config = DynamicLayoutConfig::default();
    let this = DynamicLayout {
        app,
        widget_map: Rc::new(RefCell::new(HashMap::new())),
        container: None,
        priority: Rc::new(RefCell::new(CycleOrder::new(&config))),
        activate_widget: (channel.0, Some(channel.1)),
        deactivate_widget: (channel1.0, Some(channel1.1)),
        cycle_channel: (channel2.0, Some(channel2.1)),
        config,
    };
    ROk(SabiLayoutManager_TO::from_value(this, TD_CanDowncast))
}

impl<Ord: WidgetOrderManager + 'static> SabiLayoutManager for DynamicLayout<Ord> {
    fn init(&mut self) {
        self.create_new_window();
        self.start_event_listener();
    }

    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError> {
        let conf = ron::from_str::<ron::Value>(&config)
            .with_context(|| "failed to parse config to value")
            .unwrap();
        let old_max_activities = self.config.max_activities;
        let old_max_active = self.config.max_active;
        match conf.into_rust() {
            Ok(conf) => {
                self.config = conf;
            }
            Err(err) => {
                log::error!("Failed to parse config into struct: {:#?}", err);
            }
        }
        log::debug!("current config: {:#?}", self.config);

        self.configure_container();
        if let Some(window) = self.app.windows().first() {
            self.config.window_position.reconfigure_window(window);
        }
        if old_max_active != self.config.max_active
            || old_max_activities != self.config.max_activities
        {
            self.priority
                .borrow_mut()
                .update_config_and_reset(self.config.max_active, self.config.max_activities);
            if let Some(container) = &self.container {
                let mut count = 0;
                let mut child = container.first_child();
                while let Some(widget) = child {
                    child = widget.next_sibling();
                    widget.set_visible(count < self.config.max_activities);
                    let act = widget.clone().downcast::<ActivityWidget>().unwrap();
                    act.set_mode(ActivityMode::Minimal);
                    count += 1;
                }
            }
        }
        for widget_id in self.widget_map.borrow().keys() {
            self.configure_widget(widget_id);
        }

        ROk(())
    }

    fn default_config(&self) -> RResult<RString, RBoxError> {
        let conf = DynamicLayoutConfig::default();
        match ron::ser::to_string_pretty(&conf, PrettyConfig::default()) {
            Ok(conf) => ROk(RString::from(conf)),
            Err(err) => RErr(RBoxError::new(err)),
        }
    }

    fn add_activity(&mut self, activity_id: &ActivityIdentifier, widget: SabiWidget) {
        if self.widget_map.borrow().contains_key(activity_id) {
            return;
        }
        let widget: gtk::Widget = widget.try_into().unwrap();
        let widget = match widget.downcast::<ActivityWidget>() {
            Ok(widget) => widget,
            Err(_) => {
                log::error!("widget {} is not an ActivityWidget", activity_id);
                return;
            }
        };

        self.container
            .as_ref()
            .expect("there should be a container")
            .append(&widget);
        widget.set_visible(false);
        self.widget_map
            .borrow_mut()
            .insert(activity_id.clone(), widget);
        self.configure_widget(activity_id);
        let update = self.priority.borrow_mut().add(activity_id);
        apply_ui_update(
            update,
            self.widget_map.borrow(),
            &self.container.clone().unwrap(),
            activity_id,
            None,
        );
    }
    fn get_activity(&self, activity: &ActivityIdentifier) -> ROption<SabiWidget> {
        self.widget_map
            .borrow()
            .get(activity)
            .map(|wid| SabiWidget::from(wid.clone().upcast::<gtk::Widget>()))
            .into()
    }

    fn remove_activity(&mut self, activity: &ActivityIdentifier) {
        let widget = self.widget_map.borrow_mut().remove(activity);
        if let Some(widget) = widget {
            if let Some(container) = self.container.as_ref() {
                container.remove(&widget);
                let update = self.priority.borrow_mut().remove(activity);
                apply_ui_update(
                    update,
                    self.widget_map.borrow(),
                    &self.container.clone().unwrap(),
                    activity,
                    None,
                );
                if container.first_child().is_none() {
                    // update window, for some reason if there are no children
                    // in the container, the last child stays displayed
                    if let Some(win) = self.app.windows().first() {
                        win.close();
                        self.create_new_window();
                    }
                }
            }
        }
    }
    fn list_activities(&self) -> RVec<ActivityIdentifier> {
        self.widget_map.borrow().keys().cloned().collect()
    }
    fn focus_activity(&self, activity: &ActivityIdentifier, mode_id: u8) {
        if let Some(widget) = self.widget_map.borrow().get(activity) {
            let mode = ActivityMode::try_from(mode_id).unwrap();
            widget.set_mode(mode);
            if matches!(mode, ActivityMode::Minimal | ActivityMode::Compact) {
                return;
            }
            let timeout = self.config.auto_minimize_timeout;
            let widget = widget.clone();
            glib::timeout_add_local_once(
                Duration::from_millis(timeout.try_into().unwrap()),
                move || {
                    if !widget.state_flags().contains(StateFlags::PRELIGHT) && widget.mode() == mode
                    {
                        //mouse is not on widget and mode hasn't changed
                        widget.set_mode(ActivityMode::Compact);
                    }
                },
            );
        }
    }
}

fn apply_ui_update(
    update: UiUpdate,
    widget_map: std::cell::Ref<HashMap<ActivityIdentifier, ActivityWidget>>,
    container: &gtk::Box,
    current_activity: &ActivityIdentifier,
    send_activate: Option<&UnboundedSender<ActivityIdentifier>>,
) {
    if update.is_empty() {
        return;
    }
    log::trace!("this: {current_activity}, {update}");
    let update = update.all();
    let activate = (update.to_activate, update.to_deactivate);
    let show = (update.to_show, update.to_hide);
    let move_this = (update.move_this, update.to_move_this_after);
    match activate {
        (Some(to_activate), None) => {
            let widget = widget_map.get(&to_activate).unwrap();
            if widget.mode() == ActivityMode::Minimal {
                if let Some(send) = send_activate {
                    let _ = send.send((*to_activate).clone());
                } else {
                    widget.set_mode(ActivityMode::Compact);
                }
            }
        }
        (None, Some(to_deactivate)) => {
            let widget = widget_map.get(&to_deactivate).unwrap();
            if widget.mode() != ActivityMode::Minimal {
                widget.set_mode(ActivityMode::Minimal);
            }
        }
        _ => {}
    }
    match show {
        (Some(to_show), None) => {
            let widget = widget_map.get(&to_show).unwrap();
            widget.set_visible(true);
        }
        (None, Some(to_hide)) => {
            let widget = widget_map.get(&to_hide).unwrap();
            widget.set_visible(false);
        }
        _ => {}
    }
    match move_this {
        (true, None) => {
            let this_wid = widget_map.get(current_activity).unwrap();
            container.reorder_child_after(this_wid, None::<&gtk::Widget>);
        }
        (true, Some(to_move_after)) => {
            let this_wid = widget_map.get(current_activity).unwrap();
            let other_wid = widget_map.get(&to_move_after);
            container.reorder_child_after(this_wid, other_wid);
        }
        _ => {}
    }
}

impl<Ord: WidgetOrderManager + 'static> DynamicLayout<Ord> {
    fn start_event_listener(&mut self) {
        let mut recv_activate_widget = self.activate_widget.1.take().unwrap();
        let widget_map = self.widget_map.clone();
        let priority = self.priority.clone();
        let container = self.container.clone().unwrap();
        glib::MainContext::default().spawn_local(async move {
            while let Some(id) = recv_activate_widget.recv().await {
                let widget_map = widget_map.borrow();

                let update = priority.borrow_mut().activate(&id);
                log::trace!("activate {id} ~~:~~ {:#?}", priority.borrow());
                // activate and show this
                let aw = widget_map.get(&id).unwrap();
                aw.set_mode(ActivityMode::Compact);
                // deactivate or hide other
                apply_ui_update(update, widget_map, &container, &id, None);
            }
        });

        let mut recv_deactivate_widget = self.deactivate_widget.1.take().unwrap();
        let widget_map = self.widget_map.clone();
        let priority = self.priority.clone();
        let container = self.container.clone().unwrap();
        glib::MainContext::default().spawn_local(async move {
            while let Some(id) = recv_deactivate_widget.recv().await {
                let widget_map = widget_map.borrow();

                let update = priority.borrow_mut().deactivate(&id);
                log::trace!("deactivate {id} ~~:~~ {:#?}", priority.borrow());
                // let aw = widget_map.get(&id);
                apply_ui_update(update, widget_map, &container, &id, None);
            }
        });

        let mut recv_cycle = self.cycle_channel.1.take().unwrap();
        let widget_map = self.widget_map.clone();
        let priority = self.priority.clone();
        let container = self.container.clone().unwrap();
        glib::MainContext::default().spawn_local(async move {
            while let Some(next) = recv_cycle.recv().await {
                let map = widget_map.borrow();
                let (updates, widget) = if next {
                    let updates = priority.borrow_mut().next();
                    let first = container.first_child().unwrap();
                    let first_id = map
                        .iter()
                        .find(|a| a.1.clone().upcast::<gtk::Widget>() == first)
                        .unwrap();
                    (updates, first_id)
                } else {
                    let updates = priority.borrow_mut().previous();
                    let last = container.last_child().unwrap();
                    let last_id = map
                        .iter()
                        .find(|a| a.1.clone().upcast::<gtk::Widget>() == last)
                        .unwrap();
                    (updates, last_id)
                };
                log::trace!("cycle {:#?}", priority.borrow());
                // let aw = widget_map.get(&id);

                for update in updates {
                    apply_ui_update(update, widget_map.borrow(), &container, widget.0, None);
                }
            }
        });
    }

    fn configure_widget(&self, widget_id: &ActivityIdentifier) {
        let widget_map = self.widget_map.borrow();
        let widget = widget_map.get(widget_id).unwrap();

        widget.set_valign(self.config.window_position.v_anchor.map_gtk());
        widget.set_halign(self.config.window_position.h_anchor.map_gtk());
        let mut count = 0;
        // remove old controllers
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
                    count += 1;
                }
            }
        }
        for controller in controllers.iter() {
            widget.remove_controller(controller);
        }
        // connect deactivate if it's not already connected
        if count == 0 {
            let send_deactivate = self.deactivate_widget.0.clone();
            let id = widget_id.clone();
            widget.connect_mode_notify(move |aw| {
                if aw.mode() != ActivityMode::Minimal {
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
        let id = widget_id.clone();
        press_gesture.connect_released(move |gest, _, x, y| {
            let aw = gest.widget().downcast::<ActivityWidget>().unwrap();
            if x < 0.0
                || y < 0.0
                || x > aw.size(gtk::Orientation::Horizontal).into()
                || y > aw.size(gtk::Orientation::Vertical).into()
            {
                // cycle by dragging
                if aw.mode() == ActivityMode::Minimal {
                    if let Err(err) = send_cycle.send(x < 0.0) {
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
            let timeout = self.config.auto_minimize_timeout;
            focus_controller.connect_leave(move |evt| {
                let aw = evt.widget().downcast::<ActivityWidget>().unwrap();
                let mode = aw.mode();
                if matches!(mode, ActivityMode::Minimal | ActivityMode::Compact) {
                    return;
                }
                glib::timeout_add_local_once(
                    Duration::from_millis(timeout.try_into().unwrap()),
                    move || {
                        if !aw.state_flags().contains(StateFlags::PRELIGHT) && aw.mode() == mode {
                            //mouse is not on widget and mode hasn't changed
                            aw.set_mode(ActivityMode::Compact);
                        }
                    },
                );
            });
            widget.add_controller(focus_controller);
        }
    }

    fn configure_container(&self) {
        let container = if self.container.is_none() {
            return;
        } else {
            self.container.as_ref().unwrap()
        };
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

    fn create_new_window(&mut self) {
        let window = gtk::ApplicationWindow::new(&self.app);

        let container = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        container.add_css_class("activity-container");
        let send_cycle = self.cycle_channel.0.clone();
        let cycle_gesture = gtk::GestureClick::new();
        cycle_gesture.set_button(0);
        cycle_gesture.connect_pressed(move |gest, _, _, _| {
            let button = gest.current_button();
            if button == 8 {
                // back
                send_cycle.send(true).unwrap();
            } else if button == 9 {
                // forward
                send_cycle.send(false).unwrap();
            }
        });
        container.add_controller(cycle_gesture);

        self.container = Some(container);
        window.set_child(self.container.as_ref());
        self.config
            .window_position
            .init_window(&window.clone().upcast());
        //show window
        window.present();
    }
}
