use std::{collections::HashMap, time::Duration};

use abi_stable::{
    external_types::crossbeam_channel::RSender,
    sabi_extern_fn,
    sabi_trait::TD_CanDowncast,
    std_types::{
        RBoxError,
        RResult::{self, RErr, ROk},
        RString,
    },
};
use chrono::Local;
use dynisland_core::{
    abi::{
        abi_stable, gdk, glib, gtk, log,
        module::{ActivityIdentifier, ModuleType, SabiModule, SabiModule_TO, UIServerCommand},
    },
    base_module::{BaseModule, ProducerRuntime},
    ron,
};
#[cfg(not(feature = "embedded"))]
use env_logger::Env;
use glib::object::CastNone;
use gtk::prelude::WidgetExt;
#[cfg(not(feature = "embedded"))]
use log::Level;
use ron::ser::PrettyConfig;

use crate::{
    config::{get_conf_idx, ClockConfigMain, DeClockConfigMain},
    widget::{clock::Clock, compact::Compact, get_activity},
    NAME,
};

pub struct ClockModule {
    base_module: BaseModule<ClockModule>,
    producers_rt: ProducerRuntime,
    config: ClockConfigMain,
}

#[sabi_extern_fn]
pub fn new(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError> {
    #[cfg(not(feature = "embedded"))]
    env_logger::Builder::from_env(Env::default().default_filter_or(Level::Warn.as_str())).init();
    if let Err(err) = gtk::gio::resources_register_include!("compiled.gresource") {
        return RErr(RBoxError::new(err));
    }

    let base_module = BaseModule::new(NAME, app_send.clone());
    let producers_rt = ProducerRuntime::new();
    let mut config = ClockConfigMain::default();
    config
        .windows
        .insert("".to_string(), vec![config.default_conf()]);
    let this = ClockModule {
        base_module,
        producers_rt,
        config,
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for ClockModule {
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
        match serde_json::from_str::<DeClockConfigMain>(&config) {
            Ok(conf) => {
                let mut conf = conf.into_main_config();
                if conf.windows.is_empty() {
                    conf.windows
                        .insert("".to_string(), vec![conf.default_conf()]);
                };
                self.config = conf;
            }
            Err(err) => {
                log::warn!(
                    "Failed to parse clock config into struct, using default: {:#?}",
                    err
                );
            }
        }
        log::debug!("current config: {:#?}", self.config);
        ROk(())
    }

    fn default_config(&self) -> RResult<RString, RBoxError> {
        let mut conf = ClockConfigMain::default();
        conf.windows.clear();
        match ron::ser::to_string_pretty(&conf, PrettyConfig::default()) {
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

#[allow(unused_variables)]
fn producer(module: &ClockModule) {
    let config = &module.config;

    let activities = module.base_module.registered_activities();
    let current_activities = activities.blocking_lock().list_activities();
    let desired_activities: Vec<(&str, usize)> = config
        .windows
        .iter()
        .map(|(window_name, configs)| (window_name.as_str(), configs.len()))
        .collect();
    let (to_remove, to_add) = acitvities_to_update(&current_activities, &desired_activities);
    for act in to_remove {
        log::trace!("Removing activity {}", act);
        module.base_module.unregister_activity(act.activity());
    }
    for (window, idx) in to_add {
        let act = get_activity(
            module.base_module.prop_send(),
            crate::NAME,
            "clock-activity",
            window,
            idx,
        );
        log::trace!("Adding activity {}", act.get_identifier());
        module.base_module.register_activity(act).unwrap();
    }

    let activity_list = activities.blocking_lock().list_activities();
    let mut time_list = Vec::new();
    for activity_id in activity_list {
        let activity_name = activity_id.activity();
        if let Ok(act) = activities.blocking_lock().get_activity(activity_name) {
            let conf_idx = get_conf_idx(&activity_id);
            let config = config.get_for_window(
                activity_id
                    .metadata()
                    .window_name()
                    .unwrap_or_default()
                    .as_str(),
                conf_idx,
            );
            let comp = act
                .blocking_lock()
                .get_activity_widget()
                .compact_mode_widget()
                .and_downcast::<Compact>()
                .unwrap();
            comp.set_format_24h(config.format_24h);
            let clock = act
                .blocking_lock()
                .get_activity_widget()
                .minimal_mode_widget()
                .and_downcast::<Clock>()
                .unwrap();
            clock.set_hour_hand_color(config.hour_hand_color.clone());
            clock.set_minute_hand_color(config.minute_hand_color.clone());
            clock.set_circle_color(config.circle_color.clone());
            clock.set_tick_color(config.tick_color.clone());
            clock.queue_draw();
        }
        time_list.push(
            activities
                .blocking_lock()
                .get_property_any_blocking(activity_name, "time")
                .unwrap(),
        );
    }

    module.producers_rt.handle().spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let now = Local::now();
            for time in time_list.iter() {
                time.lock().await.set(now).unwrap();
            }
        }
    });
}

pub fn acitvities_to_update<'a>(
    current: &'a Vec<ActivityIdentifier>,
    desired: &'a Vec<(&'a str, usize)>,
) -> (Vec<&'a ActivityIdentifier>, Vec<(&'a str, usize)>) {
    // (remove, add)
    //remove activities
    let mut to_remove = Vec::new();
    let mut current_windows = HashMap::new();
    for act in current {
        let idx = get_conf_idx(act);
        let window_name = act.metadata().window_name().unwrap_or_default();
        if desired
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
    for (window_name, count) in desired {
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
