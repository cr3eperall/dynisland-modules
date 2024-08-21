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
use glib::SourceId;
use gtk::prelude::*;
use log::Level;
use ron::ser::PrettyConfig;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{
    config::{ActivityMatch, DynamicLayoutConfig},
    priority_order::{cycle_order::CycleOrder, WidgetOrderManager},
};

// TODO set activity order at start
pub struct DynamicLayout<Ord: WidgetOrderManager> {
    pub(crate) app: gtk::Application,
    // widget_map: Rc<RefCell<HashMap<ActivityIdentifier, ActivityWidget>>>,
    pub(crate) cancel_minimize: Rc<RefCell<HashMap<ActivityIdentifier, SourceId>>>,
    // container: Option<gtk::Box>,
    pub(crate) order_manager: Rc<RefCell<Ord>>,
    pub(crate) activity_order: Vec<ActivityMatch>,
    pub(crate) activate_widget: (
        UnboundedSender<ActivityIdentifier>,
        Option<UnboundedReceiver<ActivityIdentifier>>,
    ),
    pub(crate) deactivate_widget: (
        UnboundedSender<ActivityIdentifier>,
        Option<UnboundedReceiver<ActivityIdentifier>>,
    ),
    pub(crate) cycle_channel: (UnboundedSender<bool>, Option<UnboundedReceiver<bool>>),
    pub(crate) config: DynamicLayoutConfig,
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
        // widget_map: Rc::new(RefCell::new(HashMap::new())),
        cancel_minimize: Rc::new(RefCell::new(HashMap::new())),
        // container: None,
        order_manager: Rc::new(RefCell::new(CycleOrder::new(&config))),
        activity_order: Vec::new(),
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
        self.activity_order = self.config.get_order();
        log::debug!("current config: {:#?}", self.config);

        self.configure_container();
        if let Some(window) = self.app.windows().first() {
            self.config.window_position.reconfigure_window(window);
        }
        // if the order manager configuration changed
        if old_max_active != self.config.max_active
            || old_max_activities != self.config.max_activities
        {
            self.order_manager
                .borrow_mut()
                .update_config(self.config.max_active, self.config.max_activities);
        }
        if self.config.reorder_on_reload {
            self.update_activity_order();
        }

        for widget_id in self.order_manager.borrow().list_activities() {
            self.configure_widget(&widget_id);
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
        if self
            .order_manager
            .borrow()
            .get_widget_map()
            .borrow()
            .contains_key(activity_id)
        {
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

        widget.set_visible(false);
        // self.container
        //     .as_ref()
        //     .expect("there should be a container")
        //     .append(&widget);

        // self.widget_map
        //     .borrow_mut()
        //     .insert(activity_id.clone(), widget);

        self.order_manager.borrow_mut().add(activity_id, widget);
        if self.config.reorder_on_add {
            self.update_activity_order();
        }

        self.configure_widget(activity_id);
    }

    fn get_activity(&self, activity: &ActivityIdentifier) -> ROption<SabiWidget> {
        self.order_manager
            .borrow()
            .get_widget_map()
            .borrow()
            .get(activity)
            .map(|wid| SabiWidget::from(wid.clone().upcast::<gtk::Widget>()))
            .into()
    }

    fn remove_activity(&mut self, activity: &ActivityIdentifier) {
        let create_new_window =
            if let Some(container) = self.order_manager.borrow().get_container().as_ref() {
                // container.remove(&widget);
                self.order_manager.borrow_mut().remove(activity);
                // update.apply(
                //     self.widget_map.borrow(),
                //     &self.container.clone().unwrap(),
                //     activity,
                // );
                if container.first_child().is_none() {
                    // update window, for some reason if there are no children
                    // in the container, the last child stays displayed
                    true
                } else {
                    false
                }
            } else {
                true
            };
        if create_new_window {
            if let Some(win) = self.app.windows().first() {
                win.close();
                self.create_new_window();
            }
        }
    }

    fn list_activities(&self) -> RVec<ActivityIdentifier> {
        self.order_manager
            .borrow()
            .list_activities()
            .iter()
            .map(|key| (**key).clone())
            .collect()
    }

    fn activity_notification(&self, activity: &ActivityIdentifier, mode_id: u8) {
        if let Some(widget) = self
            .order_manager
            .borrow()
            .get_widget_map()
            .borrow()
            .get(activity)
        {
            let mode = ActivityMode::try_from(mode_id).unwrap();
            let priority = self.order_manager.clone();
            if !priority.borrow().is_shown(activity) {
                widget.set_visible(true);
                widget.remove_css_class("hidden");
            }
            widget.set_mode(mode);
            // if matches!(mode, ActivityMode::Minimal | ActivityMode::Compact) {
            //     return;
            // }
            let timeout = self.config.auto_minimize_timeout;
            let widget = widget.clone();
            let activity = activity.clone();
            glib::timeout_add_local_once(
                Duration::from_millis(timeout.try_into().unwrap()),
                move || {
                    if priority.borrow().is_active(&activity) {
                        widget.set_mode(ActivityMode::Compact);
                    } else {
                        widget.set_mode(ActivityMode::Minimal);
                    }
                    if !priority.borrow().is_shown(&activity) {
                        widget.add_css_class("hidden");
                        widget.size_allocate(&gdk::Rectangle::new(0, 0, 50, 40), 0);
                    }
                },
            );
        } else {
            log::warn!("activity-notification: no activity named: {activity}");
        }
    }
}

impl<Ord: WidgetOrderManager + 'static> DynamicLayout<Ord> {
    fn start_event_listener(&mut self) {
        // listen to activate widget
        let mut recv_activate_widget = self.activate_widget.1.take().unwrap();
        // let widget_map = self.widget_map.clone();
        let priority = self.order_manager.clone();
        // let container = self.container.clone().unwrap();
        glib::MainContext::default().spawn_local(async move {
            while let Some(id) = recv_activate_widget.recv().await {
                // let widget_map = widget_map.borrow();

                priority.borrow_mut().activate(&id);
                log::trace!("activate {id}");
                // activate and show this
                // let aw = widget_map.get(&id).unwrap();
                // aw.set_mode(ActivityMode::Compact);
                // // deactivate or hide other
                // update.apply(widget_map, &container, &id);
            }
        });

        // listen to deactivate widget
        let mut recv_deactivate_widget = self.deactivate_widget.1.take().unwrap();
        // let widget_map = self.widget_map.clone();
        let priority = self.order_manager.clone();
        // let container = self.container.clone().unwrap();
        glib::MainContext::default().spawn_local(async move {
            while let Some(id) = recv_deactivate_widget.recv().await {
                // let widget_map = widget_map.borrow();

                priority.borrow_mut().deactivate(&id);
                log::trace!("deactivate {id}");
                // let aw = widget_map.get(&id);
                // update.apply(widget_map, &container, &id);
            }
        });

        // listen to cycle widgets
        let mut recv_cycle = self.cycle_channel.1.take().unwrap();
        // let widget_map = self.widget_map.clone();
        let priority = self.order_manager.clone();
        // let container = self.container.clone().unwrap();
        glib::MainContext::default().spawn_local(async move {
            while let Some(next) = recv_cycle.recv().await {
                // let map = widget_map.borrow();
                if next {
                    priority.borrow_mut().next();
                    // let first = container.first_child().unwrap();
                    // let first_id = map
                    //     .iter()
                    //     .find(|a| a.1.clone().upcast::<gtk::Widget>() == first)
                    //     .unwrap();
                    // (updates, first_id)
                } else {
                    priority.borrow_mut().previous();
                    // let last = container.last_child().unwrap();
                    // let last_id = map
                    //     .iter()
                    //     .find(|a| a.1.clone().upcast::<gtk::Widget>() == last)
                    //     .unwrap();
                    // (updates, last_id)
                };
                log::trace!("cycle {:#?}", priority.borrow());
                // let aw = widget_map.get(&id);

                // for update in updates {
                //     update.apply(widget_map.borrow(), &container, widget.0);
                // }
            }
        });
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
                send_cycle.send(false).unwrap();
            } else if button == 9 {
                // forward
                send_cycle.send(true).unwrap();
            }
        });
        container.add_controller(cycle_gesture);

        self.order_manager
            .borrow_mut()
            .set_container(container.clone());
        window.set_child(Some(&container));
        self.config
            .window_position
            .init_window(&window.clone().upcast());
        //show window
        window.present();
    }

    fn update_activity_order(&mut self) {
        let activities = self.order_manager.borrow().list_activities();
        let mut buckets: Vec<(ActivityMatch, Vec<&ActivityIdentifier>)> = self
            .activity_order
            .iter()
            .cloned()
            .map(|mch| (mch, Vec::new()))
            .collect();
        buckets.push((ActivityMatch::Other, Vec::new()));
        for activty in activities.iter() {
            for (mat, ref mut bucket) in buckets.iter_mut() {
                if mat.contains(activty) {
                    bucket.push(activty);
                    break;
                }
            }
        }
        let final_order: Vec<&ActivityIdentifier> = buckets
            .into_iter()
            .map(|e| e.1)
            .reduce(|mut acc, mut e| {
                acc.append(&mut e);
                acc
            })
            .unwrap_or(Vec::new());
        self.order_manager.borrow_mut().update_order(final_order);
    }
}
