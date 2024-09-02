use std::{collections::HashMap, process::Stdio};

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
#[cfg(not(feature = "embedded"))]
use env_logger::Env;
#[cfg(not(feature = "embedded"))]
use log::Level;
use ron::ser::PrettyConfig;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

use crate::{
    config::{ScriptConfig, ScriptConfigMain, ScriptConfigMainOptional},
    utils, widget, NAME,
};

pub struct ScriptModule {
    base_module: BaseModule<ScriptModule>,
    producers_rt: ProducerRuntime,
    config: ScriptConfigMain,
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

    let this = ScriptModule {
        base_module,
        producers_rt,
        config: ScriptConfigMain::default(),
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for ScriptModule {
    fn init(&self) {
        let base_module = self.base_module.clone();
        glib::MainContext::default().spawn_local(async move {
            base_module.register_producer(self::producer);
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

    fn update_config(&mut self, config: RString) -> RResult<(), RBoxError> {
        log::trace!("config: {}", config);

        match serde_json::from_str::<ScriptConfigMainOptional>(&config) {
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
        let mut conf = ScriptConfigMain::default();
        let default_conf = ScriptConfig {
            exec: "echo \"update your config file: see wiki\"".to_string(),
            ..Default::default()
        };
        conf.windows.insert("".to_string(), vec![default_conf]);
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

fn producer(module: &ScriptModule) {
    let config = &module.config;
    let rt = module.producers_rt.handle();

    let current_activities = module
        .base_module
        .registered_activities()
        .blocking_lock()
        .list_activities();
    let desired_activities: Vec<(&str, usize)> = config
        .windows
        .iter()
        .map(|(name, configs)| (name.as_str(), configs.len()))
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
            "script-activity",
            window,
            idx,
        );
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
        let image = activities
            .blocking_lock()
            .get_property_any_blocking(activity_name, "image")
            .unwrap();
        let compact_text = activities
            .blocking_lock()
            .get_property_any_blocking(activity_name, "compact-text")
            .unwrap();
        let scrolling = activities
            .blocking_lock()
            .get_property_any_blocking(activity_name, "scrolling")
            .unwrap();
        let scrolling_speed = activities
            .blocking_lock()
            .get_property_any_blocking(activity_name, "scrolling-speed")
            .unwrap();
        let max_width = activities
            .blocking_lock()
            .get_property_any_blocking(activity_name, "max-width")
            .unwrap();
        let conf = config.get(conf_idx).unwrap().clone();
        rt.spawn(async move {
            let image_type = utils::get_image_from_url(&conf.minimal_image).await;
            if let Ok(image_type) = image_type {
                image.lock().await.set(image_type).unwrap();
            }
            scrolling.lock().await.set(conf.scrolling).unwrap();
            max_width.lock().await.set(conf.max_width).unwrap();
            scrolling_speed
                .lock()
                .await
                .set(conf.scrolling_speed)
                .unwrap();
        });
        let conf = config.get(conf_idx).unwrap().clone();
        let cleanup = module.producers_rt.get_cleanup_notifier();
        rt.spawn(async move {
            if conf.exec.is_empty() {
                return;
            }
            let mut cleanup = cleanup.subscribe();

            let child = Command::new("sh")
                .arg("-c")
                .arg(conf.exec)
                .stdout(Stdio::piped())
                .spawn();
            if let Err(err) = child {
                log::error!("failed to start command: {:?}", err);
                return;
            }
            let mut child = child.unwrap();
            let reader = BufReader::new(child.stdout.take().unwrap());
            let mut lines = reader.lines();
            tokio::select! {
                _ = async {
                    while let Ok(line)=lines.next_line().await {
                        let line =match line {
                            Some(line) => line,
                            None => break,
                        };
                        compact_text.lock().await.set(line).unwrap();
                    }
                }=> {
                    log::warn!("command has exited")
                },
                _ = async {
                    let tx=cleanup.recv().await.unwrap();
                    child.kill().await.unwrap();
                    tx.send(()).unwrap();
                } => {
                    log::debug!("script cleanup done");
                }
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
