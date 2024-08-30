use std::process::Stdio;

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
use anyhow::Context;
use dynisland_core::{
    abi::{
        abi_stable, gdk, glib, gtk, log,
        module::{ModuleType, SabiModule, SabiModule_TO, UIServerCommand},
    },
    base_module::{BaseModule, ProducerRuntime},
    ron,
};
#[cfg(not(feature = "embedded"))]
use env_logger::Env;
#[cfg(not(feature = "embedded"))]
use log::Level;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
};

use crate::{utils, widget, NAME};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct ScriptConfig {
    exec: String,
    scrolling: bool,
    scrolling_speed: f32,
    /// if scrolling, it's in pixels, if not, it's in chars
    max_width: i32,
    minimal_image: String,
}

#[allow(clippy::derivable_impls)]
impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            exec: "".to_string(),
            minimal_image: String::from("image-missing-symbolic"),
            scrolling: true,
            scrolling_speed: 30.0,
            max_width: 300,
        }
    }
}

pub struct ScriptModule {
    base_module: BaseModule<ScriptModule>,
    producers_rt: ProducerRuntime,
    config: ScriptConfig,
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
        config: ScriptConfig::default(),
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for ScriptModule {
    fn init(&self) {
        let base_module = self.base_module.clone();
        glib::MainContext::default().spawn_local(async move {
            let act = widget::get_activity(base_module.prop_send(), crate::NAME, "script-activity");
            base_module.register_activity(act).unwrap();

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
        let conf = ron::from_str::<ron::Value>(&config)
            .with_context(|| "failed to parse config to value")
            .unwrap();

        match conf.into_rust() {
            Ok(conf) => {
                self.config = conf;
            }
            Err(err) => {
                log::error!("Failed to parse config into struct: {:#?}", err);
                return RErr(RBoxError::new(err));
            }
        }
        ROk(())
    }

    fn default_config(&self) -> RResult<RString, RBoxError> {
        match ron::ser::to_string_pretty(&ScriptConfig::default(), PrettyConfig::default()) {
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
fn producer(module: &ScriptModule) {
    let config = &module.config;
    let rt = module.producers_rt.handle();

    let activities = module.base_module.registered_activities();

    let image = activities
        .blocking_lock()
        .get_property_any_blocking("script-activity", "image")
        .unwrap();
    let compact_text = activities
        .blocking_lock()
        .get_property_any_blocking("script-activity", "compact-text")
        .unwrap();
    let scrolling = activities
        .blocking_lock()
        .get_property_any_blocking("script-activity", "scrolling")
        .unwrap();
    let scrolling_speed = activities
        .blocking_lock()
        .get_property_any_blocking("script-activity", "scrolling-speed")
        .unwrap();
    let max_width = activities
        .blocking_lock()
        .get_property_any_blocking("script-activity", "max-width")
        .unwrap();
    let conf = config.clone();
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
    let conf = config.clone();
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
