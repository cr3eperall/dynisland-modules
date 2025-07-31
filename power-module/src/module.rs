use std::{collections::HashMap, rc::Rc, time::Duration};

use anyhow::Context;
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
        gdk, glib,
        gtk::{self, prelude::*},
        log,
        module::{ActivityIdentifier, ModuleType, SabiModule, SabiModule_TO, UIServerCommand},
    },
    base_module::{BaseModule, ProducerRuntime},
    dynamic_activity::DynamicActivity,
};
#[cfg(not(feature = "embedded"))]
use env_logger::Env;
use ron::ser::PrettyConfig;
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};
use zbus::{zvariant::OwnedObjectPath, Connection};

use crate::{
    config::{DePowerConfigMain, PowerConfig, PowerConfigMain},
    upower::{self, device::Device, proxy::device::DeviceProxyBlocking},
    NAME,
};

pub struct PowerModule {
    pub(crate) base_module: BaseModule<PowerModule>,
    pub(crate) producers_rt: ProducerRuntime,
    pub(crate) config: PowerConfigMain,
    pub(crate) connection: zbus::Connection,
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
    let mut config = PowerConfigMain::default();
    // if the module was loaded we want at least one activity
    config
        .windows
        .insert("".to_string(), vec![PowerConfig::default()]);

    let connection = match producers_rt.handle().block_on(zbus::Connection::system()) {
        Ok(c) => c,
        Err(err) => return RErr(RBoxError::new(err)),
    };

    let this = PowerModule {
        base_module,
        producers_rt,
        config,
        connection,
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for PowerModule {
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

    fn cli_command(&self, command: RString) -> RResult<RString, RBoxError> {
        let command_vec = command.as_str().split(" ").collect::<Vec<_>>();
        match command_vec.first() {
            Some(&"list") => {
                let conn = zbus::blocking::Connection::system().unwrap();
                let pw = upower::proxy::upower::UPowerProxyBlocking::new(&conn).unwrap();
                let devices = pw.enumerate_devices().unwrap();
                let mut res = String::new();
                for dev in devices {
                    let device_proxy: zbus::Result<DeviceProxyBlocking<'_>> = (|| {
                        DeviceProxyBlocking::builder(&conn)
                            .destination("org.freedesktop.UPower")?
                            .path(&dev)?
                            .build()
                    })(
                    );
                    let device_proxy = match device_proxy {
                        Ok(proxy) => proxy,
                        Err(err) => {
                            return RErr(RBoxError::from_fmt(&format!(
                                "Error while reading device {}: {}",
                                dev.as_str(),
                                err
                            )))
                        }
                    };
                    let path = device_proxy.native_path().unwrap_or(String::from("?"));
                    let model = device_proxy.model().unwrap_or(String::from("?"));
                    let serial = device_proxy.serial().unwrap_or(String::from("?"));
                    let percentage = device_proxy.percentage().unwrap_or(-1.0);
                    res+=&format!("Device name: {path}, model: {model}, serial: {serial}, percentage: {percentage}%\n");
                }
                ROk(res.into())
            }
            Some(&"help") | None => {
                #[rustfmt::skip]
                return ROk(
r"Commands: 
    list: list the available batteries"
                .into());
            }
            _ => RErr(RBoxError::from_fmt(&format!(
                "Unknown command: {}",
                command
            ))),
        }
    }

    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError> {
        log::trace!("config: {}", config);
        match serde_json::from_str::<DePowerConfigMain>(&config) {
            Ok(conf) => {
                let mut conf = conf.into_main_config();
                if conf.windows.is_empty() {
                    conf.windows
                        .insert("".to_string(), vec![conf.default_conf()]);
                }
                log::debug!("new config: {:#?}", conf.windows);
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
        let config = PowerConfigMain::default();
        // if the config has child_only properties we need to add a default config to the windows
        // config.windows.insert("".to_string(), vec![PowerConfig::default()]);
        match ron::ser::to_string_pretty(&config, PrettyConfig::default()) {
            Ok(conf) => ROk(RString::from(conf)),
            Err(err) => RErr(RBoxError::new(err)),
        }
    }

    fn restart_producers(&self) {
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
fn producer(module: &PowerModule) {
    let config = &module.config;

    let activities = module.base_module.registered_activities();
    let current_activities = activities.blocking_lock().list_activities();
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
    let mut activity_map: HashMap<ActivityIdentifier, Rc<Mutex<DynamicActivity>>> = HashMap::new();
    let reg = module.base_module.registered_activities();
    let reg_lock = reg.blocking_lock();
    for activity_id in reg_lock.list_activities() {
        let act = reg_lock.get_activity(activity_id.activity()).unwrap();
        let id = act.blocking_lock().get_identifier().clone();
        activity_map.insert(id, act);
    }
    drop(reg_lock);
    for (window_name, idx) in to_add {
        // create a new dynamic activity and register it
        let act = crate::widget::get_activity(
            module.base_module.prop_send(),
            crate::NAME,
            "power-activity",
            window_name,
            idx,
        );
        let id = act.get_identifier();
        log::trace!("Adding activity {}", id);
        let act = Rc::new(Mutex::new(act));
        let id = act.blocking_lock().get_identifier().clone();
        activity_map.insert(id, act);
    }
    let mut activity_vec = Vec::new();
    for (activity_id, act) in activity_map.iter() {
        activity_vec.push((activity_id.clone(), act.clone()));
    }

    // activity register manager
    let app_send = module.base_module.app_send();
    let registered_activities = module.base_module.registered_activities();
    let (register_tx, mut register_rx) =
        tokio::sync::mpsc::unbounded_channel::<(ActivityIdentifier, bool)>();
    let conf = config.clone();
    glib::MainContext::default().spawn_local(async move {
        while let Some((activity_id, register)) = register_rx.recv().await {
            let dyn_act = match activity_map.get(&activity_id) {
                Some(act) => act,
                None => {
                    log::error!("activity not found: {}", activity_id);
                    continue;
                }
            };
            let activity_lock = dyn_act.blocking_lock();
            let widget = activity_lock.get_activity_widget();
            let id = activity_lock.get_identifier();

            drop(activity_lock);
            let mut reg_act_lock = registered_activities.lock().await;
            if register {
                if reg_act_lock.get_activity(activity_id.activity()).is_err() {
                    if let Err(err) = app_send.send(UIServerCommand::AddActivity {
                        activity_id: id,
                        widget: widget.upcast::<gtk::Widget>().into(),
                    }) {
                        log::error!("failed to send add activity from app: {}", err);
                        continue;
                    }
                    reg_act_lock.insert_activity(dyn_act.clone()).unwrap();
                    let config = conf.get_for_window(
                        activity_id
                            .metadata()
                            .window_name()
                            .unwrap_or_default()
                            .as_str(),
                        get_conf_idx(&activity_id),
                    );
                    config.apply_to(dyn_act);
                }
            } else {
                if reg_act_lock.get_activity(activity_id.activity()).is_ok() {
                    if let Err(err) = app_send.send(UIServerCommand::RemoveActivity {
                        activity_id: id.clone(),
                    }) {
                        log::error!("failed to send remove activity from app: {}", err);
                        continue;
                    }
                    reg_act_lock.remove_activity(&id).unwrap();
                }
            }
        }
    });

    // the updates need to be done on a different thread, this way the main ui thread is not blocked
    let rt = module.producers_rt.clone();

    for (activity_id, dyn_act) in activity_vec {
        let idx = get_conf_idx(&activity_id);
        let window_name = activity_id.metadata().window_name().unwrap_or_default();
        let activity_config = config.get_for_window(&window_name, idx);
        let config = config.get_for_window(&window_name, idx);

        // set the configs
        config.apply_to(&dyn_act);
        // get the properties
        let percentage_prop = dyn_act
            .blocking_lock()
            .get_property_any("percentage")
            .unwrap();
        let charging_prop = dyn_act
            .blocking_lock()
            .get_property_any("charging")
            .unwrap();
        let time_to = dyn_act.blocking_lock().get_property_any("time-to").unwrap();
        let points = dyn_act.blocking_lock().get_property_any("points").unwrap();

        let mut device_rx = device_change_updater(
            &rt,
            activity_id.clone(),
            &module.connection.clone(),
            !config.hide_if_missing,
            register_tx.clone(),
            &activity_config.battery,
        );

        rt.handle().spawn({
            let conn = module.connection.clone();
            async move {
            loop{
                let device = match device_rx.recv().await {
                    Some(dev) => dev,
                    None => {
                        break;
                    },
                };
                let dev_path = match device {
                    Some(dev) => dev,
                    None => {
                        log::debug!("Device not found: {}", activity_config.battery);
                        tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
                        continue;
                    }
                };
                let device_obj=Device::from_path(&conn, dev_path).await.unwrap();
                let result: anyhow::Result<()> = async {
                    loop {
                        // log::debug!("looping");
                        if !device_rx.is_empty(){
                            return Ok(());
                        }
                        // if config.battery.is_empty() {
                        //     // use display and drop device_rx
                        //     // let dev_type=display.type_().await.unwrap();
                        //     // let level = display.battery_level().await.unwrap();
                        //     // let state = display.state().await.unwrap();
                        //     // let tech = display.technology().await.unwrap();
                        //     // let percentage = display.proxy.percentage().await.unwrap();
                        //     // log::debug!("Display: \nBattery level: {:?}, \ntype: {:?}, \nstate: {:?}, \ntechnology: {:?}, \npercentage: {:?}", level, dev_type, state, tech, percentage);

                        //     // tokio::time::sleep(std::time::Duration::from_millis(3500)).await;
                        //     continue;
                        // }
                        // TODO keep the device enumeration and update the state if a device was added/removed

                        let name = device_obj.proxy.native_path().await.with_context(||"getting name")?;
                        let dev_type=device_obj.type_().await.with_context(||"getting dev_type")?;
                        let level = device_obj.battery_level().await.with_context(||"getting level")?;
                        let state = device_obj.state().await.with_context(||"getting state")?;
                        let tech = device_obj.technology().await.with_context(||"getting tech")?;
                        let percentage = device_obj.proxy.percentage().await.with_context(||"getting percentage")?;
                        let time_to_full = device_obj.time_to_full().await.unwrap_or(Duration::ZERO);
                        let time_to_empty = device_obj.time_to_empty().await.unwrap_or(Duration::ZERO);
                        let hist = if device_obj.proxy.has_history().await.with_context(||"getting has_history")?{
                            device_obj.get_history(upower::device::HistoryType::Charge, config.max_duration_secs as u32, config.max_duration_secs/60).await.with_context(||"getting history(charge)")?
                        }else{
                            Vec::new()
                        };
                        //TODO find if there is a way to detect when the device was connected(there is a bug in the graph because after the device reaches 100% there are no more history records)
                        let rate = if device_obj.proxy.has_history().await.with_context(||"getting has_history")?{
                            device_obj.get_history(upower::device::HistoryType::Rate, 100, 10).await.with_context(||"getting history(rate)")?
                        } else{
                            Vec::new()
                        };
                        if name == config.battery{
                            // log::debug!("Found device: {:?}, hist: {:#?}, rate: {:#?}", name, hist, rate);
                            percentage_prop.lock().await.set(percentage/100.0).unwrap();
                            charging_prop.lock().await.set(matches!(state,upower::device::State::Charging)).unwrap();
                            time_to.lock().await.set((state, time_to_empty.as_secs(), time_to_full.as_secs())).unwrap();
                            points.lock().await.set(hist).unwrap();
                        }
                        log::debug!("Name: {name} \nBattery level: {:?}, \ntype: {:?}, \nstate: {:?}, \ntechnology: {:?}, \npercentage: {:?}", level, dev_type, state, tech, percentage);

                        tokio::time::sleep(std::time::Duration::from_millis(3500)).await;
                    }
                }.await;
                log::debug!("Device updater for {} returned in loop: {result:#?}", activity_id);

                tokio::time::sleep(std::time::Duration::from_millis(4000)).await;
            }
            log::debug!("Device updater for {} stopped", activity_id);
        }});
    }
}

pub fn device_change_updater(
    rt: &ProducerRuntime,
    activity_id: ActivityIdentifier,
    conn: &Connection,
    keep_registered: bool,
    register_tx: UnboundedSender<(ActivityIdentifier, bool)>,
    device_name: &str,
) -> UnboundedReceiver<Option<OwnedObjectPath>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let conn = conn.clone();
    let device_name = device_name.to_string();
    let mut cleanup_tx = rt.get_cleanup_notifier();
    rt.handle().spawn(async move {
        let pw = upower::proxy::upower::UPowerProxy::new(&conn).await.unwrap();
        if device_name.is_empty(){
            tx.send(Some(pw.get_display_device().await.unwrap())).unwrap();
            register_tx.send((activity_id.clone(), true)).unwrap();
            return;
        }
        let mut old_device = Option::None;
        let mut found_device = Option::None;
        tokio::select! {
            clean = cleanup_tx.recv() => {
                if let Ok(sender)= clean {
                    drop(register_tx);
                    sender.send(()).unwrap();
                    return;
                }
            }
            _ = async {
                loop {
                    let devices = pw.enumerate_devices().await.unwrap_or_else(|err|{log::warn!("error enumerating devices: {:?}",err);Vec::new()} );
                    let mut found = false;
                    for dev in devices.iter(){
                        let device = upower::device::Device::from_path(&conn, dev.clone()).await.unwrap();
                        let name = device.proxy.native_path().await.unwrap();
                        // log::debug!("Device: {:?}", name);

                        if name == device_name || device_name == dev.as_str() {
                            old_device = found_device;
                            found_device = Some(dev.clone());
                            found=true;
                            break;
                        }
                    }
                    if found{
                        if old_device != found_device{
                            let res = tx.send(found_device.clone());
                            match res {
                                Ok(_) => {
                                    if old_device.is_none(){
                                        register_tx.send((activity_id.clone(), true)).unwrap();
                                    }
                                }
                                Err(_) => {
                                    register_tx.send((activity_id.clone(), keep_registered)).unwrap();
                                }
                            }
                        }
                    }else {
                        old_device = found_device;
                        found_device = None;
                        //TODO maybe add config to use display device if the battery is not found
                        if old_device.is_some(){
                            if keep_registered{
                                let _ = tx.send(Some(pw.get_display_device().await.unwrap()));
                            }else{
                                let _ = tx.send(None);
                            }
                            // log::debug!("sent none");
                        }
                        register_tx.send((activity_id.clone(), keep_registered)).unwrap();
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(4000)).await;
                }
            } => {
                log::warn!("Device change updater for {}(device_name: {}) stopped", activity_id, device_name);
                return;
            }
        }
    });
    rx
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
