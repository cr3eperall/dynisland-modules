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
use anyhow::Result;
use dynisland_core::{
    abi::{
        abi_stable, gdk, glib, gtk,
        layout::{LayoutManagerType, SabiLayoutManager, SabiLayoutManager_TO},
        log,
        module::ActivityIdentifier,
        SabiApplication, SabiWidget,
    },
    graphics::activity_widget::{boxed_activity_mode::ActivityMode, ActivityWidget},
    ron,
};
#[cfg(not(feature = "embedded"))]
use env_logger::Env;
use glib::SourceId;
use gtk::prelude::*;
#[cfg(not(feature = "embedded"))]
use log::Level;
use ron::ser::PrettyConfig;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::{
    config::{
        self, ActivityMatch, DynamicLayoutConfig, DynamicLayoutConfigMain,
        DynamicLayoutConfigMainOptional,
    },
    priority_order::cycle_order::CycleOrder,
};

pub struct DynamicLayout {
    pub(crate) app: gtk::Application,
    pub(crate) cancel_minimize: Rc<RefCell<HashMap<ActivityIdentifier, SourceId>>>,
    pub(crate) order_managers: Rc<RefCell<HashMap<String, Rc<RefCell<CycleOrder>>>>>,
    pub(crate) activate_widget: (
        UnboundedSender<ActivityIdentifier>,
        Option<UnboundedReceiver<ActivityIdentifier>>,
    ),
    pub(crate) deactivate_widget: (
        UnboundedSender<ActivityIdentifier>,
        Option<UnboundedReceiver<ActivityIdentifier>>,
    ),
    pub(crate) cycle_channel: (
        UnboundedSender<(String, bool)>,
        Option<UnboundedReceiver<(String, bool)>>,
    ),
    pub(crate) config: DynamicLayoutConfigMain,
}

#[sabi_extern_fn]
pub fn new(app: SabiApplication) -> RResult<LayoutManagerType, RBoxError> {
    #[cfg(not(feature = "embedded"))]
    env_logger::Builder::from_env(Env::default().default_filter_or(Level::Warn.as_str())).init();

    let app = app.try_into().unwrap();
    let channel = tokio::sync::mpsc::unbounded_channel();
    let channel1 = tokio::sync::mpsc::unbounded_channel();
    let channel2 = tokio::sync::mpsc::unbounded_channel();
    let config = DynamicLayoutConfigMain::default();
    let this = DynamicLayout {
        app,
        cancel_minimize: Rc::new(RefCell::new(HashMap::new())),
        order_managers: Rc::new(RefCell::new(
            HashMap::<String, Rc<RefCell<CycleOrder>>>::new(),
        )),
        activate_widget: (channel.0, Some(channel.1)),
        deactivate_widget: (channel1.0, Some(channel1.1)),
        cycle_channel: (channel2.0, Some(channel2.1)),
        config,
    };
    ROk(SabiLayoutManager_TO::from_value(this, TD_CanDowncast))
}

impl SabiLayoutManager for DynamicLayout {
    fn init(&mut self) {
        self.update_windows();
        self.start_event_listener();
    }

    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError> {
        log::trace!("config: {:#?}", config);
        let mut conf_opt = DynamicLayoutConfigMainOptional::default();
        match serde_json::from_str(&config) {
            Ok(conf) => {
                conf_opt = conf;
            }
            Err(err) => {
                log::error!(
                    "Failed to parse config into struct, using default: {:#?}",
                    err
                );
            }
        }
        self.config = conf_opt.into_main_config();
        log::debug!("current config: {:#?}", self.config);

        if self.app.windows().first().is_some() {
            self.update_windows();
            for window_name in self.config.windows.keys() {
                let window = self
                    .order_managers
                    .borrow()
                    .get(window_name)
                    .unwrap()
                    .borrow()
                    .get_window();
                self.config
                    .get_for_window(window_name)
                    .window_position
                    .reconfigure_window(&window);
            }
        }
        for ord in self.order_managers.borrow().iter() {
            self.configure_container(&ord.0);
            ord.1.borrow_mut().update_config(
                self.config.max_active,
                self.config.get_for_window(ord.0).max_activities,
            );
            if self.config.get_for_window(ord.0).reorder_on_reload {
                Self::update_activity_order(ord.1, &self.config.get_for_window(ord.0));
            }

            for widget_id in ord.1.borrow().list_activities() {
                self.configure_widget(&widget_id);
            }
        }

        ROk(())
    }

    fn default_config(&self) -> RResult<RString, RBoxError> {
        let conf = DynamicLayoutConfigMain::default();
        match ron::ser::to_string_pretty(&conf, PrettyConfig::default()) {
            Ok(conf) => ROk(RString::from(conf)),
            Err(err) => RErr(RBoxError::new(err)),
        }
    }

    fn add_activity(&mut self, activity_id: &ActivityIdentifier, widget: SabiWidget) {
        let widget: gtk::Widget = widget.try_into().unwrap();
        let widget = match widget.downcast::<ActivityWidget>() {
            Ok(widget) => widget,
            Err(_) => {
                log::error!("widget {} is not an ActivityWidget", activity_id);
                return;
            }
        };

        widget.set_visible(false);
        let window_name = self.get_window_name(activity_id);
        let order_manager = self.order_managers.borrow();
        let ord = order_manager.get(window_name.as_str()).unwrap();
        ord.borrow_mut().add(activity_id, widget);
        let config = self.config.get_for_window(window_name.as_str());
        if config.reorder_on_add {
            Self::update_activity_order(ord, &config);
        }

        self.configure_widget(activity_id);
    }

    fn get_activity(&self, activity: &ActivityIdentifier) -> ROption<SabiWidget> {
        match Self::find_widget(&self.order_managers.borrow(), activity) {
            Some((widget, _)) => {
                let widget: gtk::Widget = widget.clone().upcast();
                ROption::RSome(widget.into())
            }
            None => ROption::RNone,
        }
    }

    fn remove_activity(&mut self, activity: &ActivityIdentifier) {
        let (widget, _) = match Self::find_widget(&self.order_managers.borrow(), activity) {
            Some(wid) => wid,
            None => {
                return;
            }
        };
        self.remove_activity_from_ord(activity, widget).unwrap();
    }

    fn list_activities(&self) -> RVec<ActivityIdentifier> {
        self.order_managers
            .borrow()
            .values()
            .flat_map(|ord| ord.borrow().list_activities())
            .fold(RVec::new(), |mut vec, id| {
                vec.push((*id).clone());
                vec
            })
    }

    fn list_windows(&self) -> RVec<RString> {
        self.order_managers
            .borrow()
            .keys()
            .fold(RVec::new(), |mut vec, id| {
                vec.push(id.clone().into());
                vec
            })
    }

    fn activity_notification(
        &self,
        activity: &ActivityIdentifier,
        mode_id: u8,
        duration: ROption<u64>,
    ) {
        if let Some((widget, window_name)) =
            Self::find_widget(&self.order_managers.borrow(), activity)
        {
            let mode = ActivityMode::try_from(mode_id).unwrap();
            let priority = self
                .order_managers
                .borrow()
                .get(&window_name)
                .unwrap()
                .clone();
            if !priority.borrow().is_shown(activity) {
                widget.set_visible(true);
                widget.remove_css_class("hidden");
            }
            widget.set_mode(mode);
            // if matches!(mode, ActivityMode::Minimal | ActivityMode::Compact) {
            //     return;
            // }
            let timeout = duration.unwrap_or(
                self.config
                    .get_for_window(&window_name)
                    .auto_minimize_timeout
                    .try_into()
                    .unwrap_or(config::DEFAULT_AUTO_MINIMIZE_TIMEOUT as u64),
            );
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

impl DynamicLayout {
    fn get_window_name(&self, activity_id: &ActivityIdentifier) -> String {
        let requested_window = activity_id.metadata().window_name().unwrap_or_default();
        if self.order_managers.borrow().contains_key(&requested_window) {
            requested_window
        } else {
            "".to_string()
        }
    }
    fn start_event_listener(&mut self) {
        // listen to activate widget
        let mut recv_activate_widget = self.activate_widget.1.take().unwrap();
        // let widget_map = self.widget_map.clone();
        let order_managers = self.order_managers.clone();
        // let container = self.container.clone().unwrap();
        glib::MainContext::default().spawn_local(async move {
            while let Some(id) = recv_activate_widget.recv().await {
                // let widget_map = widget_map.borrow();
                let (_, window_name) = Self::find_widget(&order_managers.borrow(), &id).unwrap();
                let order_managers = &order_managers.borrow();
                let ord = order_managers.get(&window_name).unwrap();
                ord.borrow_mut().activate(&id);
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
        let order_managers = self.order_managers.clone();
        // let container = self.container.clone().unwrap();
        glib::MainContext::default().spawn_local(async move {
            while let Some(id) = recv_deactivate_widget.recv().await {
                // let widget_map = widget_map.borrow();
                let (_, window_name) = Self::find_widget(&order_managers.borrow(), &id).unwrap();
                let order_managers = &order_managers.borrow();
                let ord = order_managers.get(&window_name).unwrap();
                ord.borrow_mut().deactivate(&id);
                log::trace!("deactivate {id}");
                // let aw = widget_map.get(&id);
                // update.apply(widget_map, &container, &id);
            }
        });

        // listen to cycle widgets
        let mut recv_cycle = self.cycle_channel.1.take().unwrap();
        // let widget_map = self.widget_map.clone();
        let order_managers = self.order_managers.clone();
        // let container = self.container.clone().unwrap();
        glib::MainContext::default().spawn_local(async move {
            while let Some((window_name, next)) = recv_cycle.recv().await {
                // let map = widget_map.borrow();
                log::trace!("cycle {}", window_name);
                let order_managers = &order_managers.borrow();
                let ord = order_managers.get(&window_name).unwrap();
                if next {
                    ord.borrow_mut().next();
                    // let first = container.first_child().unwrap();
                    // let first_id = map
                    //     .iter()
                    //     .find(|a| a.1.clone().upcast::<gtk::Widget>() == first)
                    //     .unwrap();
                    // (updates, first_id)
                } else {
                    ord.borrow_mut().previous();
                    // let last = container.last_child().unwrap();
                    // let last_id = map
                    //     .iter()
                    //     .find(|a| a.1.clone().upcast::<gtk::Widget>() == last)
                    //     .unwrap();
                    // (updates, last_id)
                };
                log::trace!("cycle {:#?}", ord.borrow());
                // let aw = widget_map.get(&id);

                // for update in updates {
                //     update.apply(widget_map.borrow(), &container, widget.0);
                // }
            }
        });
    }

    pub(super) fn find_widget(
        order_managers: &HashMap<String, Rc<RefCell<CycleOrder>>>,
        activity: &ActivityIdentifier,
    ) -> Option<(ActivityWidget, String)> {
        for (window_name, ord) in order_managers.iter() {
            let ord = ord.borrow();
            let widget_map = ord.get_widget_map();
            if let Some(widget) = widget_map.borrow().get(activity) {
                return Some((widget.clone(), window_name.clone()));
            };
        }
        None
    }

    fn update_windows(&mut self) {
        let mut orphan_widgets: Vec<(ActivityIdentifier, ActivityWidget)> = Vec::new();
        // remove windows that are no longer in the config
        let mut windows_to_remove: Vec<String> = Vec::new();
        for (window_name, ord) in self.order_managers.borrow().iter() {
            if !self.config.windows.contains_key(&window_name.to_string()) {
                let mut widgets = Vec::new();
                for child in ord
                    .borrow()
                    .get_container()
                    .observe_children()
                    .iter::<glib::Object>()
                    .flatten()
                {
                    let widget = child.downcast::<gtk::Widget>().unwrap();
                    if let Some((id, widget)) = ord
                        .borrow()
                        .get_widget_map()
                        .borrow()
                        .iter()
                        .find(|(_, w)| *w == &widget)
                    {
                        orphan_widgets.push(((**id).clone(), widget.clone()));
                    }
                    widgets.push(widget);
                }
                for widget in widgets {
                    ord.borrow().get_container().remove(&widget);
                }
                windows_to_remove.push(window_name.clone());
                ord.borrow().window.close();
            }
        }
        for window_name in windows_to_remove {
            self.order_managers.borrow_mut().remove(&window_name);
            log::trace!("removing window no longer in config {}", window_name);
        }
        // create new windows
        let existing_windows: Vec<String> = self.order_managers.borrow().keys().cloned().collect();
        let mut windows_to_create: Vec<String> = Vec::new();
        for window_name in self.config.windows.keys() {
            if !existing_windows.contains(window_name) {
                windows_to_create.push(window_name.clone());
            }
        }
        for window_name in windows_to_create {
            log::trace!("creating new window {}", window_name);
            self.create_new_window(&window_name);
        }
        for (widget_id, widget) in orphan_widgets {
            let window_name = self.get_window_name(&widget_id);
            self.order_managers
                .borrow()
                .get(&window_name)
                .unwrap()
                .borrow_mut()
                .add(&widget_id, widget);
            log::trace!("readding orphaned widget {}", widget_id);
        }
        let mut to_update: Vec<(ActivityIdentifier, ActivityWidget)> = Vec::new();
        for (current_window, ord) in self.order_managers.borrow().iter() {
            for (id, widget) in ord.borrow().get_widget_map().borrow().iter() {
                if let Some(desired_window) = id.metadata().window_name() {
                    if desired_window != *current_window
                        && self.config.windows.contains_key(&desired_window)
                    {
                        to_update.push(((**id).clone(), widget.clone()));
                    }
                }
            }
        }
        for (id, widget) in to_update {
            self.remove_activity_from_ord(&id, widget.clone()).unwrap();
            let window_name = self.get_window_name(&id);
            let order_managers = self.order_managers.borrow();
            let ord = order_managers.get(&window_name).unwrap();
            ord.borrow_mut().add(&id, widget);
            log::trace!("moving widget {} to correct window", id);
        }
        log::debug!("updated windows");
    }

    fn create_new_window(&mut self, window_name: &str) {
        if self.order_managers.borrow().contains_key(window_name) {
            return;
        }
        if !self.config.windows.contains_key(window_name) {
            return;
        }
        let window = gtk::ApplicationWindow::new(&self.app);
        window.set_title(Some(window_name));
        let container = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        container.add_css_class("activity-container");
        let send_cycle = self.cycle_channel.0.clone();
        let cycle_gesture = gtk::GestureClick::new();
        cycle_gesture.set_button(0);
        let window_name1 = window_name.to_string();
        cycle_gesture.connect_pressed(move |gest, _, _, _| {
            let button = gest.current_button();
            if button == 8 {
                // back
                send_cycle.send((window_name1.clone(), false)).unwrap();
            } else if button == 9 {
                // forward
                send_cycle.send((window_name1.clone(), true)).unwrap();
            }
        });
        container.add_controller(cycle_gesture);
        let window = window.upcast();
        if !self.order_managers.borrow().contains_key(window_name) {
            self.order_managers.borrow_mut().insert(
                window_name.to_string(),
                Rc::new(RefCell::new(CycleOrder::new(
                    &self.config.get_for_window(window_name),
                    &window,
                    &container,
                ))),
            );
        }
        self.configure_container(&window_name);
        window.set_child(Some(&container));
        self.config
            .get_for_window(&window_name)
            .window_position
            .init_window(&window.clone().upcast());
        //show window
        window.present();
    }

    fn remove_activity_from_ord(
        &mut self,
        activity: &ActivityIdentifier,
        widget: ActivityWidget,
    ) -> Result<()> {
        let widget_container = match widget.parent().unwrap().downcast::<gtk::Box>() {
            Ok(parent) => parent,
            Err(_) => {
                log::warn!(
                    "Error removing {activity:?} from {}: parent is not a Box",
                    crate::NAME
                );
                anyhow::bail!(
                    "Error removing {activity:?} from {}: parent is not a Box",
                    crate::NAME
                );
            }
        };
        let mut window_name = String::new();
        for (name, ord) in self.order_managers.borrow().iter() {
            let container = ord.borrow().get_container();
            if container == widget_container {
                ord.borrow_mut().remove(activity);
                if container.first_child().is_some() {
                    return Ok(());
                }
                ord.borrow().get_window().close();
            } else {
                continue;
            };
            window_name = name.clone();
            break;
        }
        self.order_managers.borrow_mut().remove(&window_name);
        log::debug!("removing empty window {}", window_name);
        self.create_new_window(&window_name);
        Ok(())
    }

    fn update_activity_order(order: &Rc<RefCell<CycleOrder>>, config: &DynamicLayoutConfig) {
        let activities = order.borrow().list_activities();
        let mut buckets: Vec<(ActivityMatch, Vec<&ActivityIdentifier>)> = config
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
        order.borrow_mut().update_order(final_order);
    }
}
