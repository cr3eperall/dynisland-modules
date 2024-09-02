use std::collections::HashMap;

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
use dynisland_core::{
    abi::{
        abi_stable, gdk, glib, gtk, log,
        module::{ActivityIdentifier, ModuleType, SabiModule, SabiModule_TO, UIServerCommand},
    },
    base_module::{BaseModule, ProducerRuntime},
    ron,
};
use env_logger::Env;
use log::Level;
use ron::ser::PrettyConfig;

use super::NAME;
use crate::{
    config::{DeExampleConfigMain, ExampleConfigMain},
    widget,
};

pub struct ExampleModule {
    base_module: BaseModule<ExampleModule>,
    producers_rt: ProducerRuntime,
    config: ExampleConfigMain,
}

#[sabi_extern_fn]
pub fn new(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError> {
    env_logger::Builder::from_env(Env::default().default_filter_or(Level::Warn.as_str())).init();
    if let Err(err) = gtk::gio::resources_register_include!("compiled.gresource") {
        return RErr(RBoxError::new(err));
    }

    let base_module = BaseModule::new(NAME, app_send);
    let this = ExampleModule {
        base_module,
        producers_rt: ProducerRuntime::new(),
        config: ExampleConfigMain::default(),
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for ExampleModule {
    #[allow(clippy::let_and_return)]
    fn init(&self) {
        let base_module = self.base_module.clone();
        glib::MainContext::default().spawn_local(async move {
            base_module.register_producer(producer);
        });
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

    #[allow(clippy::let_and_return)]
    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError> {
        log::trace!("config: {}", config);

        match serde_json::from_str::<DeExampleConfigMain>(&config) {
            Ok(conf) => {
                self.config = conf.into_main_config();
            }
            Err(err) => {
                log::error!(
                    "Failed to parse config into struct, using default: {:#?}",
                    err
                );
            }
        }
        log::debug!("current config: {:#?}", self.config);
        ROk(())
    }

    fn default_config(&self) -> RResult<RString, RBoxError> {
        match ron::ser::to_string_pretty(&ExampleConfigMain::default(), PrettyConfig::default()) {
            Ok(conf) => ROk(RString::from(conf)),
            Err(err) => RErr(RBoxError::new(err)),
        }
    }

    #[allow(clippy::let_and_return)]
    fn restart_producers(&self) {
        self.producers_rt.shutdown_blocking();
        self.producers_rt.reset_blocking();
        for producer in self
            .base_module
            .registered_producers()
            .blocking_lock()
            .iter()
        {
            producer(self)
        }
    }
}

pub(crate) fn get_conf_idx(id: &ActivityIdentifier) -> usize {
    id.metadata()
        .additional_metadata()
        .unwrap()
        .split("|")
        .find(|s| s.starts_with("instance="))
        .unwrap()
        .split("=")
        .last()
        .unwrap()
        .parse::<usize>()
        .unwrap()
}

#[allow(unused_variables)]
fn producer(module: &ExampleModule) {
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
        module.base_module.unregister_activity(act);
    }
    for (window, idx) in to_add {
        let act = widget::get_activity(
            module.base_module.prop_send(),
            crate::NAME,
            "example-activity",
            window,
            idx,
        );
        log::trace!("Adding activity {}", act.get_identifier());
        module.base_module.register_activity(act).unwrap();
    }
    let activities = module.base_module.registered_activities();
    let activity_list = activities.blocking_lock().list_activities();
    for activity in activity_list {
        let activity_name = activity.activity();
        let config = config.get_for_window(
            activity
                .metadata()
                .window_name()
                .unwrap_or_default()
                .as_str(),
        );
        let conf_idx = get_conf_idx(&activity);
        let label = activities
            .blocking_lock()
            .get_property_any_blocking(activity_name, "comp-label")
            .unwrap();
        let scrolling_text = activities
            .blocking_lock()
            .get_property_any_blocking(activity_name, "scrolling-label-text")
            .unwrap();
        let rolling_char = activities
            .blocking_lock()
            .get_property_any_blocking(activity_name, "rolling-char")
            .unwrap();

        let conf = config.get(conf_idx).unwrap().clone();
        // log::debug!("starting task");
        module.producers_rt.handle().spawn(async move {
            // log::debug!("task started");
            loop {
                rolling_char.lock().await.set('0').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(conf.duration)).await;
                rolling_char.lock().await.set('1').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(conf.duration)).await;
                rolling_char.lock().await.set('2').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(conf.duration)).await;
                rolling_char.lock().await.set('3').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(conf.duration)).await;
                rolling_char.lock().await.set('4').unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(conf.duration)).await;

                scrolling_text
                    .lock()
                    .await
                    .set("Scrolling Label but longer".to_string())
                    .unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(17000)).await;

                scrolling_text.lock().await.set("Hi".to_string()).unwrap();
                tokio::time::sleep(tokio::time::Duration::from_millis(4000)).await;
            }
        });
    }
}

pub fn acitvities_to_update<'a>(
    current: &'a Vec<ActivityIdentifier>,
    desired: &'a Vec<(&'a str, usize)>,
) -> (Vec<&'a str>, Vec<(&'a str, usize)>) {
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
            to_remove.push(act.activity());
        }
        let idx: usize = *current_windows.get(&window_name).unwrap_or(&0).max(&idx);
        current_windows.insert(window_name, idx);
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
