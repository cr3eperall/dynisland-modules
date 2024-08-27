use std::vec;

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
use env_logger::Env;
use log::Level;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use super::{widget, NAME};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct ExampleConfig {
    pub number_of_widgets: u32,
    pub int: i32,
    pub string: String,
    pub vec: Vec<String>,
    pub duration: u64,
}

impl Default for ExampleConfig {
    fn default() -> Self {
        Self {
            number_of_widgets: 1,
            int: 0,
            string: String::from("Example1"),
            vec: vec![String::from("Example2"), String::from("Example3")],
            duration: 400,
        }
    }
}
pub struct ExampleModule {
    base_module: BaseModule<ExampleModule>,
    producers_rt: ProducerRuntime,
    config: ExampleConfig,
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
        config: ExampleConfig::default(),
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for ExampleModule {
    #[allow(clippy::let_and_return)]
    fn init(&self) {
        let widget_count = self.config.number_of_widgets;
        let base_module = self.base_module.clone();
        glib::MainContext::default().spawn_local(async move {
            let act = widget::get_activity(base_module.prop_send(), NAME, "exampleActivity0");
            //register activity and data producer
            base_module.register_activity(act).unwrap();
            base_module.register_producer(producer);
            for i in 1..widget_count {
                //create activity
                let act = widget::get_activity(
                    base_module.prop_send(),
                    NAME,
                    format!("exampleActivity{}", i).as_str(),
                );

                //register activity and data producer
                base_module.register_activity(act).unwrap();
            }
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
        let conf = ron::from_str::<ron::Value>(&config)
            .with_context(|| "failed to parse config to value")
            .unwrap();

        self.config = conf
            .into_rust()
            .with_context(|| "failed to parse config to struct")
            .unwrap();
        ROk(())
    }

    fn default_config(&self) -> RResult<RString, RBoxError> {
        match ron::ser::to_string_pretty(&ExampleConfig::default(), PrettyConfig::default()) {
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

#[allow(unused_variables)]
fn producer(module: &ExampleModule) {
    let config: &ExampleConfig = &module.config;

    let registered_activities = module.base_module.registered_activities();
    let registered_activities_lock = registered_activities.blocking_lock();
    let label = registered_activities_lock
        .get_property_any_blocking("exampleActivity0", "comp-label")
        .unwrap();
    let scrolling_text = registered_activities_lock
        .get_property_any_blocking("exampleActivity0", "scrolling-label-text")
        .unwrap();
    let rolling_char = registered_activities_lock
        .get_property_any_blocking("exampleActivity0", "rolling-char")
        .unwrap();

    let config = config.clone();
    // debug!("starting task");
    module.producers_rt.handle().spawn(async move {
        // debug!("task started");
        loop {
            rolling_char.lock().await.set('0').unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;
            rolling_char.lock().await.set('1').unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;
            rolling_char.lock().await.set('2').unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;
            rolling_char.lock().await.set('3').unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;
            rolling_char.lock().await.set('4').unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(config.duration)).await;

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
