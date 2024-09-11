use std::{collections::HashMap, rc::Rc, sync::Arc, time::Duration};

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
        abi_stable, gdk,
        glib::{self, subclass::types::ObjectSubclassIsExt},
        gtk::{self, prelude::Cast},
        log,
        module::{ActivityIdentifier, ModuleType, SabiModule, SabiModule_TO, UIServerCommand},
    },
    base_module::{BaseModule, ProducerRuntime},
    dynamic_activity::DynamicActivity,
};
#[cfg(not(feature = "embedded"))]
use env_logger::Env;
#[cfg(not(feature = "embedded"))]
use log::Level;
use ron::ser::PrettyConfig;
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex, MutexGuard,
};

use crate::{
    config::{acitvities_to_update, get_conf_idx, DeMusicConfigMain, MusicConfig, MusicConfigMain},
    player_info::{MprisPlayer, MprisProgressEvent},
    producer_tasks::{action_task, ui_update_task, visualizer_task, wait_for_new_player_task},
    widget::{self, expanded::Expanded, UIAction},
    NAME,
};

pub(crate) const CHECK_DELAY: u64 = 5000;

pub struct MusicModule {
    base_module: BaseModule<MusicModule>,
    producers_rt: ProducerRuntime,
    config: MusicConfigMain,
}

#[sabi_extern_fn]
pub fn new(app_send: RSender<UIServerCommand>) -> RResult<ModuleType, RBoxError> {
    #[cfg(not(feature = "embedded"))]
    env_logger::Builder::from_env(Env::default().default_filter_or(Level::Warn.as_str()))
        .filter(Some("reqwest"), log::LevelFilter::Warn)
        .init();
    if let Err(err) = gtk::gio::resources_register_include!("compiled.gresource") {
        return RErr(RBoxError::new(err));
    }

    let base_module = BaseModule::new(NAME, app_send.clone());
    let producers_rt = ProducerRuntime::new();
    let mut config = MusicConfigMain::default();
    config
        .windows
        .insert("".to_string(), vec![config.default_conf()]);
    let this = MusicModule {
        base_module,
        producers_rt,
        config,
    };
    ROk(SabiModule_TO::from_value(this, TD_CanDowncast))
}

impl SabiModule for MusicModule {
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
        match serde_json::from_str::<DeMusicConfigMain>(&config) {
            Ok(conf) => {
                let mut conf = conf.into_main_config();
                if conf.windows.is_empty() {
                    conf.windows
                        .insert("".to_string(), vec![conf.default_conf()]);
                };
                self.config = conf;
            }
            Err(err) => {
                log::error!(
                    "Failed to parse config into struct, using default: {:#?}",
                    err
                );
                return RErr(RBoxError::new(err));
            }
        }
        log::debug!("current config: {:#?}", self.config);
        ROk(())
    }

    fn default_config(&self) -> RResult<RString, RBoxError> {
        let mut conf = MusicConfigMain::default();
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
fn producer(module: &MusicModule) {
    let config = &module.config;
    let rt = module.producers_rt.clone();

    // get activities to add/remove
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
    let mut activity_map: HashMap<ActivityIdentifier, Rc<Mutex<DynamicActivity>>> = HashMap::new();
    let reg = module.base_module.registered_activities();
    let reg_lock = reg.blocking_lock();
    for activity_id in reg_lock.list_activities() {
        let act = reg_lock.get_activity(activity_id.activity()).unwrap();
        let id = act.blocking_lock().get_identifier().clone();
        activity_map.insert(id, act);
    }
    drop(reg_lock);
    for (window, idx) in to_add {
        let act = widget::get_activity(
            module.base_module.prop_send(),
            crate::NAME,
            "music-activity",
            window,
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

    for (activity_id, act) in activity_vec {
        let activity_name = activity_id.activity();
        let act_lock = act.blocking_lock();
        let conf_idx = get_conf_idx(&activity_id);
        let config = config.get_for_window(
            activity_id
                .metadata()
                .window_name()
                .unwrap_or_default()
                .as_str(),
            conf_idx,
        );

        // set configs
        let scrolling_label_speed = act_lock.get_property_any("scrolling-label-speed").unwrap();
        rt.handle().spawn(async move {
            scrolling_label_speed
                .lock()
                .await
                .set(config.scrolling_label_speed)
                .unwrap();
        });

        let (player_change_tx, _) =
            tokio::sync::broadcast::channel::<(MprisPlayer, UnboundedSender<Duration>)>(4);
        let (event_rx_tx, event_rx_rx) =
            tokio::sync::mpsc::channel::<UnboundedReceiver<MprisProgressEvent>>(4);
        let (player_quit_tx, player_quit_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

        // start visualizer updater
        start_visualizer_updater(&rt, &act_lock, &config);

        // start ui updater
        start_ui_updater(
            &rt,
            config.clone(),
            player_quit_tx.clone(),
            &player_change_tx,
            event_rx_rx,
            act_lock,
        );

        // start ui action executor
        let action_rx = {
            let expanded_mode = act
                .blocking_lock()
                .get_activity_widget()
                .expanded_mode_widget()
                .unwrap()
                .downcast::<Expanded>()
                .unwrap();
            expanded_mode.imp().action_rx.clone()
        };
        start_ui_action_executor(&rt, player_quit_tx, &player_change_tx, action_rx);

        // start new player updater
        let preferred_player = config.preferred_player.clone();
        start_player_change_updater(
            &rt,
            preferred_player,
            activity_id,
            register_tx.clone(),
            player_change_tx,
            event_rx_tx,
            player_quit_rx,
            config.use_fallback_player,
        );
    }
}

fn start_player_change_updater(
    rt: &ProducerRuntime,
    preferred_player: String,
    activity_id: ActivityIdentifier,
    register_tx: UnboundedSender<(ActivityIdentifier, bool)>,
    player_change_tx: tokio::sync::broadcast::Sender<(MprisPlayer, UnboundedSender<Duration>)>,
    event_rx_tx: tokio::sync::mpsc::Sender<UnboundedReceiver<MprisProgressEvent>>,
    mut player_quit_rx: UnboundedReceiver<()>,
    use_fallback_player: bool,
) {
    let mut cleanup = rt.get_cleanup_notifier();
    rt.handle().spawn(async move {
        loop {
            let player = match MprisPlayer::new(&preferred_player, !use_fallback_player) {
                Ok(pl) => {
                    let get_player = pl.get_player();
                    let player = get_player.lock().unwrap();
                    let name = player.bus_name_player_name_part();
                    log::trace!("found player: {name}");
                    register_tx.send((activity_id.clone(), true)).unwrap();
                    pl
                }
                Err(err) => {
                    log::trace!("no player found: {}", err);
                    register_tx.send((activity_id.clone(), false)).unwrap();
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(CHECK_DELAY))=> {

                        },
                        clean = cleanup.recv() => {
                            if let Ok(sender)= clean {
                                drop(register_tx);
                                sender.send(()).unwrap();
                                return;
                            }
                        }
                    };
                    continue;
                }
            };
            let current_player_name = {
                let player = player.get_player();
                let player = player.lock().unwrap();
                player.bus_name_player_name_part().to_string()
            };
            let (event_rx, seek_tx) = player
                .start_progress_tracker(Duration::from_millis(200), Duration::from_millis(1000))
                .unwrap();
            player_change_tx.send((player.clone(), seek_tx)).unwrap();
            event_rx_tx.send(event_rx).await.unwrap();
            tokio::select! {
                _ = wait_for_new_player_task(&current_player_name, &preferred_player) =>{
                    log::trace!("time to change player");
                },
                _ = player_quit_rx.recv() => {
                    log::trace!("player has quit");
                },
                clean = cleanup.recv() => {
                    if let Ok(sender)= clean {
                        drop(register_tx);
                        sender.send(()).unwrap();
                        return;
                    }
                }
            }
        }
    });
}

fn start_ui_action_executor(
    rt: &ProducerRuntime,
    player_quit_tx: UnboundedSender<()>,
    player_change_tx: &tokio::sync::broadcast::Sender<(MprisPlayer, UnboundedSender<Duration>)>,
    action_rx: Arc<Mutex<UnboundedReceiver<UIAction>>>,
) {
    let mut player_change_rx = player_change_tx.subscribe();
    rt.handle().spawn(async move {
        let (mut player, mut seek_tx) = match player_change_rx.recv().await {
            Ok(data) => data,
            Err(_) => {
                return;
            }
        };
        let mut set_by_change = true;
        loop {
            if !set_by_change {
                (player, seek_tx) = player_change_rx.recv().await.unwrap();
            }
            tokio::select! {
                res = action_task(player.clone(), seek_tx.clone(), action_rx.clone()) => {
                    if let Err(_) = res {
                        player_quit_tx.send(()).unwrap();
                        set_by_change=false;
                    }
                },
                player_change = player_change_rx.recv() => {
                    if let Ok((new_player, new_seek_tx)) = player_change {
                        player = new_player;
                        seek_tx = new_seek_tx;
                        set_by_change=true;
                    } else {
                        log::debug!("player change task failed");
                        break;
                    }
                }
            }
        }
    });
}

fn start_visualizer_updater(
    rt: &ProducerRuntime,
    activities_lock: &MutexGuard<'_, DynamicActivity>,
    config: &MusicConfig,
) {
    let visualizer_data = activities_lock.get_property_any("visualizer-data").unwrap();
    let conf = config.clone();
    let cleanup = rt.get_cleanup_notifier();
    rt.handle().spawn(async move {
        visualizer_task(&conf.cava_visualizer_script, visualizer_data, cleanup).await;
        log::debug!("visualizer task has exited");
    });
}

fn start_ui_updater(
    rt: &ProducerRuntime,
    config: MusicConfig,
    player_quit_tx: UnboundedSender<()>,
    player_change_tx: &tokio::sync::broadcast::Sender<(MprisPlayer, UnboundedSender<Duration>)>,
    mut event_rx_rx: tokio::sync::mpsc::Receiver<UnboundedReceiver<MprisProgressEvent>>,
    act_lock: MutexGuard<'_, DynamicActivity>,
) {
    let metadata = act_lock.get_property_any("music-metadata").unwrap();
    let time = act_lock.get_property_any("music-time").unwrap();
    let playback = act_lock.get_property_any("playback-status").unwrap();
    let album_art = act_lock.get_property_any("album-art").unwrap();
    let visualizer_gradient = act_lock.get_property_any("visualizer-gradient").unwrap();

    let mut change_rx = player_change_tx.subscribe();
    rt.handle().spawn(async move {
        let mut player = match change_rx.recv().await {
            Ok(pl) => pl,
            Err(_) => {
                return;
            }
        }
        .0;
        let mut event_rx = event_rx_rx.recv().await.unwrap();
        let mut set_by_change = true;
        loop {
            if !set_by_change {
                player = change_rx.recv().await.unwrap().0;
                event_rx = event_rx_rx.recv().await.unwrap();
            }
            tokio::select! {
                res = ui_update_task(
                    player.clone(),
                    &config,
                    &mut event_rx,
                    &time,
                    &metadata,
                    &playback,
                    &visualizer_gradient,
                    &album_art,
                ) => {
                    if let Err(_) = res {
                        player_quit_tx.send(()).unwrap();
                        set_by_change=false;
                    }
                },
                player_change = change_rx.recv() => {
                    if let Ok((new_player, _)) = player_change {
                        player = new_player;
                        set_by_change=true;
                        event_rx = event_rx_rx.recv().await.unwrap();
                    } else {
                        log::debug!("player change task failed");
                        break;
                    }
                }
            }
        }
    });
}
