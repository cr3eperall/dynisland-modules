use std::time::Duration;

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
use chrono::Local;
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
use glib::object::CastNone;
use gtk::prelude::WidgetExt;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::{
    widget::{clock::Clock, compact::Compact, get_activity},
    NAME,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct ClockConfig {
    format_24h: bool,
    hour_hand_color: String,
    minute_hand_color: String,
    tick_color: String,
    circle_color: String,
}

#[allow(clippy::derivable_impls)]
impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            format_24h: true,
            hour_hand_color: String::from("white"),
            minute_hand_color: String::from("white"),
            circle_color: String::from("lightgray"),
            tick_color: String::from("lightgray"),
        }
    }
}

pub struct ClockModule {
    base_module: BaseModule<ClockModule>,
    producers_rt: ProducerRuntime,
    config: ClockConfig,
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

    let this = ClockModule {
        base_module,
        producers_rt,
        config: ClockConfig::default(),
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for ClockModule {
    fn init(&self) {
        let base_module = self.base_module.clone();
        glib::MainContext::default().spawn_local(async move {
            base_module.register_producer(self::producer);
            let activity = get_activity(base_module.prop_send(), crate::NAME, "clock-activity");
            base_module.register_activity(activity).unwrap();
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
        match ron::ser::to_string_pretty(&ClockConfig::default(), PrettyConfig::default()) {
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

    if let Ok(act) = activities.blocking_lock().get_activity("clock-activity") {
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

    let time = activities
        .blocking_lock()
        .get_property_any_blocking("clock-activity", "time")
        .unwrap();

    module.producers_rt.handle().spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            time.lock().await.set(Local::now()).unwrap();
        }
    });
    // todo!("do stuff")
}
