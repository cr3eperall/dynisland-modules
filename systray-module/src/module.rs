use std::{collections::HashMap, sync::Arc};

use dynisland_core::{
    abi::{
        abi_stable::{
            external_types::crossbeam_channel::RSender,
            sabi_extern_fn,
            sabi_trait::TD_CanDowncast,
            std_types::{
                RBoxError,
                RResult::{self, RErr, ROk},
                RString,
            },
        },
        gdk::{self, prelude::*, ModifierType},
        glib::{self, subclass::types::ObjectSubclassIsExt},
        gtk::{self, prelude::*},
        log,
        module::{ActivityIdentifier, ModuleType, SabiModule, SabiModule_TO, UIServerCommand},
    },
    base_module::{BaseModule, ProducerRuntime},
    cast_dyn_any,
    graphics::activity_widget::{boxed_activity_mode::ActivityMode, ActivityWidget},
};
#[cfg(not(feature = "embedded"))]
use env_logger::Env;
use ron::ser::PrettyConfig;
use tokio::sync::Mutex;

use crate::{
    config::{DeSystrayConfigMain, MenuHeightMode, SystrayConfig, SystrayConfigMain},
    item_menu_tasks, item_tasks,
    status_notifier::{self, item::Item, menu::Menu, watcher::Watcher},
    widget::{
        compact::{Compact, ItemAction},
        expanded::Expanded,
    },
    NAME,
};

pub struct SystrayModule {
    pub(crate) base_module: BaseModule<SystrayModule>,
    pub(crate) producers_rt: ProducerRuntime,
    pub(crate) config: SystrayConfigMain,
    pub(crate) connection: zbus::Connection,
    pub(crate) items: Arc<Mutex<HashMap<String, (Item, Option<Menu>)>>>,
}

#[sabi_extern_fn]
pub fn new(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError> {
    #[cfg(not(feature = "embedded"))]
    env_logger::Builder::from_env(Env::default().default_filter_or(log::Level::Warn.as_str()))
        .init();
    if let Err(err) = gtk::gio::resources_register_include!("compiled.gresource") {
        return RErr(RBoxError::new(err));
    }

    let base_module = BaseModule::new(NAME, app_send.clone());
    let producers_rt = ProducerRuntime::new();
    let mut config = SystrayConfigMain::default();
    // if the module was loaded we want at least one activity
    config
        .windows
        .insert("".to_string(), vec![SystrayConfig::default()]);

    let connection = match producers_rt.handle().block_on(zbus::Connection::session()) {
        Ok(c) => c,
        Err(err) => return RErr(RBoxError::new(err)),
    };

    let this = SystrayModule {
        base_module,
        producers_rt,
        config,
        connection: connection,
        items: Arc::new(Mutex::new(HashMap::new())),
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for SystrayModule {
    // register the producers and the default css provider
    // this is called after the module is created but before gtk is initialized
    // so any code that uses gtk should be spawned on the main context
    fn init(&self) {
        self.base_module.register_producer(self::producer);

        let fallback_provider = gtk::CssProvider::new();
        let css = grass::from_string(include_str!("../default.scss"), &grass::Options::default())
            .unwrap();
        fallback_provider.load_from_string(&css);
        glib::MainContext::default().spawn_local(async move {
            gtk::style_context_add_provider_for_display(
                &gdk::Display::default().unwrap(),
                &fallback_provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        });
    }

    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError> {
        log::trace!("config: {}", config);
        match serde_json::from_str::<DeSystrayConfigMain>(&config) {
            Ok(conf) => {
                let mut conf = conf.into_main_config();
                if conf.windows.is_empty() {
                    conf.windows
                        .insert("".to_string(), vec![conf.default_conf()]);
                };
                self.config = conf;
            }
            Err(err) => {
                log::error!("Failed to parse config into struct: {:#?}", err);
                return RErr(RBoxError::new(err));
            }
        }
        log::debug!("current config: {:#?}", self.config);
        ROk(())
    }

    fn default_config(&self) -> RResult<RString, RBoxError> {
        let config = SystrayConfigMain::default();
        // if the config has child_only properties we need to add a default config to the windows
        // config.windows.insert("".to_string(), vec![SystrayConfig::default()]);
        match ron::ser::to_string_pretty(&config, PrettyConfig::default()) {
            Ok(conf) => ROk(RString::from(conf)),
            Err(err) => RErr(RBoxError::new(err)),
        }
    }

    fn restart_producers(&self) {
        log::debug!("shutting down producers");
        self.producers_rt.shutdown_blocking();
        self.producers_rt.reset_blocking();
        //restart producers
        for producer in self
            .base_module
            .registered_producers()
            .blocking_lock()
            .iter()
        {
            producer(self);
        }
    }
}

// this function is called from the main gtk ui thread,
// so you can update gtk properties here
// (but not in the producer runtime, to do that you need to use dynamic properties).
// This function should only setup the runtime to update dynamic properties
// and should return as soon as possible
#[allow(unused_variables)]
fn producer(module: &SystrayModule) {
    let config = &module.config;
    let activity_map = module.base_module.registered_activities();

    let current_activities = activity_map.blocking_lock().list_activities();
    let desired_activities: Vec<(&str, usize)> = config
        .windows
        .iter()
        .map(|(window_name, activities)| (window_name.as_str(), activities.len()))
        .collect();

    let (to_remove, to_add) = activities_to_update(&current_activities, &desired_activities);
    for activity_id in to_remove {
        // unregister the activity to remove
        module
            .base_module
            .unregister_activity(activity_id.activity());
    }
    for (window_name, idx) in to_add {
        // create a new dynamic activity and register it
        let actvity = crate::widget::get_activity(
            module.base_module.prop_send(),
            crate::NAME,
            "systray-activity",
            window_name,
            idx,
        );
        module.base_module.register_activity(actvity).unwrap();
    }

    // now that only the configured activities remain, we can update their properties
    let activity_list = activity_map.blocking_lock().list_activities();

    // the updates need to be done on a different thread, this way the main ui thread is not blocked
    let rt = module.producers_rt.clone();
    let (main_context_cleanup_tx, main_context_cleanup_rx) =
        tokio::sync::broadcast::channel::<()>(1);
    let mut rt_cleanup = rt.get_cleanup_notifier();
    rt.handle().spawn(async move {
        let cleanup = rt_cleanup.recv().await.unwrap();
        main_context_cleanup_tx.send(()).unwrap();
        cleanup.send(()).unwrap();
    });
    let conn = module.connection.clone();

    let (add_item_tx, add_item_rx): (
        tokio::sync::broadcast::Sender<(String, Item)>,
        tokio::sync::broadcast::Receiver<(String, Item)>,
    ) = tokio::sync::broadcast::channel(16);
    let (remove_item_tx, remove_item_rx): (
        tokio::sync::broadcast::Sender<String>,
        tokio::sync::broadcast::Receiver<String>,
    ) = tokio::sync::broadcast::channel(16);

    update_shared_items_task(&rt, module, &add_item_rx, &remove_item_rx);

    for activity_id in activity_list {
        let idx = get_conf_idx(&activity_id);
        let window_name = activity_id.metadata().window_name().unwrap_or_default();
        let activity_config = config.get_for_window(&window_name, idx);

        let dyn_act = activity_map
            .blocking_lock()
            .get_activity(activity_id.activity())
            .unwrap();

        let menu_height_mode = dyn_act
            .blocking_lock()
            .get_property_any("height-mode")
            .unwrap();
        menu_height_mode
            .blocking_lock()
            .set(MenuHeightMode::from(
                activity_config.menu_height_mode.as_str(),
            ))
            .unwrap();

        let activity_widget = dyn_act.blocking_lock().get_activity_widget();
        let compact = activity_widget
            .compact_mode_widget()
            .unwrap()
            .downcast::<Compact>()
            .unwrap();
        let expanded = activity_widget
            .expanded_mode_widget()
            .unwrap()
            .downcast::<Expanded>()
            .unwrap();
        let expanded_action_rx = expanded.imp().action_rx.clone();
        let minimal_count = dyn_act.blocking_lock().get_property_any("count").unwrap();

        item_tasks::start_add_item_task(
            &main_context_cleanup_rx,
            &add_item_rx,
            minimal_count.clone(),
            &compact,
        );

        item_tasks::start_remove_item_task(&remove_item_rx, minimal_count.clone(), &compact);

        let action_rx = compact.imp().action_rx.clone();

        start_item_action_task(module, activity_widget, action_rx);

        item_menu_tasks::start_item_menu_action_task(&rt, module, expanded_action_rx);

        glib::MainContext::default().spawn_local({
            let rt = rt.clone();
            let mut cleanup = main_context_cleanup_rx.resubscribe();
            async move {
                cleanup.recv().await.unwrap();
                compact.clear_items();
                let mut minimal_count = minimal_count.lock().await;
                let count = cast_dyn_any!(minimal_count.get(), i32).unwrap();
                minimal_count.set(0).unwrap();
            }
        });
    }

    register_host(rt, conn, add_item_tx, remove_item_tx);
}

fn start_item_action_task(
    module: &SystrayModule,
    activity_widget: ActivityWidget,
    action_rx: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<(String, ItemAction)>>>,
) {
    glib::MainContext::default().spawn_local({
        let items = module.items.clone();
        let activity_widget = activity_widget.clone();
        let expanded = activity_widget
            .expanded_mode_widget()
            .unwrap()
            .downcast::<Expanded>()
            .unwrap();
        async move {
            let mut action_rx = action_rx.lock().await;
            while let Some((id, action)) = action_rx.recv().await {
                let item = match items.lock().await.get(&id) {
                    Some(it) => it.0.clone(),
                    None => continue,
                };
                let item_is_menu = item.item_is_menu().await.unwrap_or(false);
                // FIXME this is relative to the widget, not the screen, but gtk4 doesn't have a way to get coordinates relative to the screen
                let (mouse_x, mouse_y) =
                    get_mouse_position_relative_to_window(&activity_widget.clone().upcast());

                match (action, item_is_menu) {
                    (ItemAction::Clicked(gdk::BUTTON_PRIMARY), false) => {
                        if item.activate(mouse_x, mouse_y).await.is_err() {
                            let menu = item_menu_tasks::get_menu(&item).await;
                            if let Some(menu) = menu {
                                let layout = menu.get_layout_root().await.unwrap();
                                expanded.set_layout(layout, Some(Vec::new()), id.clone());
                                activity_widget.set_mode(ActivityMode::Expanded);

                                item_menu_tasks::start_item_menu_update_task(&expanded, menu, &id)
                                    .await;
                            }
                        }
                    }
                    (ItemAction::Clicked(gdk::BUTTON_MIDDLE), _) => {
                        let _ = item.secondary_activate(mouse_x, mouse_y).await;
                    }
                    (ItemAction::Clicked(gdk::BUTTON_SECONDARY), _)
                    | (ItemAction::Clicked(gdk::BUTTON_PRIMARY), true) => {
                        let menu = item_menu_tasks::get_menu(&item).await;
                        if let Some(menu) = menu {
                            let layout = menu.get_layout_root().await.unwrap();
                            expanded.set_layout(layout, Some(Vec::new()), id.clone());
                            activity_widget.set_mode(ActivityMode::Expanded);

                            item_menu_tasks::start_item_menu_update_task(&expanded, menu, &id)
                                .await;
                        } else {
                            let _ = item.context_menu(mouse_x, mouse_y).await;
                        }
                    }
                    (ItemAction::Scrolled(direction_horizontal, delta), _) => {
                        let _ = item.scroll(delta, direction_horizontal).await;
                    }
                    (ItemAction::Clicked(btn), _) => {
                        log::debug!("unknown button {} detected for item: {}", btn, id)
                    }
                }
            }
        }
    });
}

fn get_mouse_position_relative_to_window(widget: &gtk::Widget) -> (i32, i32) {
    let mut parent: gtk::Widget = widget.clone().upcast();
    while let Some(par) = parent.parent() {
        parent = par;
    }
    let display = gdk::Display::default().unwrap();
    let pointer = display.default_seat().unwrap().pointer().unwrap();
    let surface = parent.native().unwrap().surface().unwrap();
    // FIXME this is relative to the window, not the screen, but gtk4 doesn't have a way to get coordinates relative to the screen
    let (mouse_x, mouse_y, _modifiers) =
        surface
            .device_position(&pointer)
            .unwrap_or((0.0, 0.0, ModifierType::empty()));
    (mouse_x as i32, mouse_y as i32)
}

fn update_shared_items_task(
    rt: &ProducerRuntime,
    module: &SystrayModule,
    add_item_rx: &tokio::sync::broadcast::Receiver<(String, Item)>,
    remove_item_rx: &tokio::sync::broadcast::Receiver<String>,
) {
    rt.handle().spawn({
        let mut add_item_rx = add_item_rx.resubscribe();
        let items = module.items.clone();
        let mut cleanup = rt.get_cleanup_notifier();
        async move {
            while let Some(_) = tokio::select! {
                res = add_item_rx.recv() => {
                    if let Ok((item_id, item)) = res{
                        let menu = item_menu_tasks::get_menu(&item).await;
                        items.lock().await.insert(item_id, (item, menu));
                        Some(())
                    }else{
                        None
                    }
                }
                c = cleanup.recv() => {
                    c.unwrap().send(()).unwrap();
                    None
                }
            } {}
        }
    });
    rt.handle().spawn({
        let mut remove_item_rx = remove_item_rx.resubscribe();
        let items: Arc<Mutex<HashMap<String, (Item, Option<Menu>)>>> = module.items.clone();
        let mut cleanup = rt.get_cleanup_notifier();
        async move {
            while let Some(_) = tokio::select! {
                res = remove_item_rx.recv() => {
                    if let Ok(item_id) = res{
                        items.lock().await.remove(&item_id);
                        Some(())
                    }else{
                        None
                    }
                }
                c = cleanup.recv() => {
                    c.unwrap().send(()).unwrap();
                    None
                }
            } {}
        }
    });
}

fn register_host(
    rt: ProducerRuntime,
    conn: zbus::Connection,
    add_item_tx: tokio::sync::broadcast::Sender<(String, Item)>,
    remove_item_tx: tokio::sync::broadcast::Sender<String>,
) {
    rt.handle().spawn({
        let conn = conn.clone();
        let hdl = rt.handle();
        async move {
            let watcher = Watcher::new(hdl);
            let _ = watcher.attach_to(&conn).await;
            let (_, snw) = status_notifier::host::register_as_host(&conn)
                .await
                .unwrap();
            let _ = status_notifier::host::run_host(add_item_tx, remove_item_tx, &snw).await;
        }
    });
}

/// Returns the activities to add and remove to get from the current state to the desired state
///
/// # Arguments
///
/// * `current_state` - The current state of the activities,
/// this can be either the activities that are currently registered (`module.base_module.registered_activities().blocking_lock().list_activities()`) or
/// the activities from the last config update if you saved them in the module
///
/// * `desired_state` - The desired state of the activities,
/// it's a vector of tuples where the first element is the window name and the second element is the number of activities for that window
///
/// # Returns
///
/// `(to_remove, to_add)`
///
/// * `to_remove` - A vector of activities that should be removed
/// * `to_add` - A vector of tuples where the first element is the window name and the second element is the instance number of the activity
pub fn activities_to_update<'a>(
    current_state: &'a Vec<ActivityIdentifier>,
    desired_state: &'a Vec<(&'a str, usize)>,
) -> (Vec<&'a ActivityIdentifier>, Vec<(&'a str, usize)>) {
    // remove activities
    let mut to_remove = Vec::new();
    let mut current_windows = HashMap::new();
    for act in current_state {
        let idx = get_conf_idx(act);
        let window_name = act.metadata().window_name().unwrap_or_default();
        if desired_state
            .iter()
            .find(|(name, count)| *name == window_name && *count > idx)
            .is_none()
        {
            to_remove.push(act);
        }
        let max_idx: usize = *current_windows.get(&window_name).unwrap_or(&0).max(&idx);
        current_windows.insert(window_name, max_idx);
    }
    //add activities
    let mut to_add = Vec::new();
    for (window_name, count) in desired_state {
        if !current_windows.contains_key(&window_name.to_string()) {
            for i in 0..*count {
                to_add.push((*window_name, i));
            }
        } else {
            let current_idx = current_windows.get(*window_name).unwrap() + 1;
            for i in current_idx..*count {
                to_add.push((*window_name, i));
            }
        }
    }
    (to_remove, to_add)
}

/// Returns the instance number of the activity
pub(crate) fn get_conf_idx(id: &ActivityIdentifier) -> usize {
    id.metadata()
        .additional_metadata("instance")
        .unwrap()
        .parse::<usize>()
        .unwrap()
}
