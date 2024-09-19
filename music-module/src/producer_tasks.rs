use std::{process::Stdio, sync::Arc, time::Duration};

use anyhow::Result;
use dynisland_core::{abi::log, cast_dyn_any, dynamic_property::DynamicPropertyAny};
use mpris::{DBusError, TrackID};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::{
        broadcast::Receiver,
        mpsc::{UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};

use crate::{
    config::MusicConfig,
    module::CHECK_DELAY,
    player_info::{MprisPlayer, MprisProgressEvent},
    utils,
    widget::{visualizer, UIAction, UIPlaybackStatus},
};

pub(crate) async fn visualizer_task(
    command: &str,
    visualizer_data: Arc<Mutex<DynamicPropertyAny>>,
    mut cleanup: Receiver<UnboundedSender<()>>,
) {
    let command = command.to_string();
    let child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .spawn();
    if let Err(err) = child {
        log::error!("failed to start visualizer command: {:?}", err);
        return;
    }
    let mut child = child.unwrap();
    let reader = BufReader::new(child.stdout.take().unwrap());
    let mut lines = reader.lines();
    tokio::select! {
        _ = async {
            while let Ok(line)=lines.next_line().await {
                let line =match line {
                    Some(line) => line/* .strip_prefix('[').unwrap().strip_suffix(']').unwrap().to_string() */,
                    None => break,
                };
                visualizer_data.lock().await.set(visualizer::parse_input(&line)).unwrap();
            }
        }=> {
            log::warn!("visualizer command has exited")
        },
        _ = async {
            let tx=cleanup.recv().await.unwrap();
            child.kill().await.unwrap();
            tx.send(()).unwrap();
        } => {
            log::debug!("visualizer cleanup done");
        }
    }
}

pub(crate) async fn ui_update_task(
    player: MprisPlayer,
    config: &MusicConfig,
    event_rx: &mut UnboundedReceiver<MprisProgressEvent>,
    time: &Arc<Mutex<DynamicPropertyAny>>,
    metadata: &Arc<Mutex<DynamicPropertyAny>>,
    playback: &Arc<Mutex<DynamicPropertyAny>>,
    visualizer_gradient: &Arc<Mutex<DynamicPropertyAny>>,
    album_art: &Arc<Mutex<DynamicPropertyAny>>,
) -> Result<()> {
    let player_metadata = player.get_metadata();
    if let Ok(player_metadata) = player_metadata {
        set_album_art(
            player_metadata.art_url(),
            &config.default_album_art_url,
            &album_art,
            &visualizer_gradient,
        )
        .await;
    }
    let mut track_id = player
        .get_current_track_id()
        .unwrap_or(TrackID::no_track())
        .to_string();
    track_id.push_str(
        &player
            .get_metadata()
            .map(|meta| meta.title().unwrap_or("").to_owned())
            .unwrap_or(String::from("")),
    );

    //init UI
    while let Some(event) = event_rx.recv().await {
        match event {
            crate::player_info::MprisProgressEvent::PlayerQuit => {
                log::debug!("player has quit");

                set_album_art(
                    None,
                    &config.default_album_art_url,
                    &album_art,
                    &visualizer_gradient,
                )
                .await;

                time.lock()
                    .await
                    .set::<(Duration, Duration)>((Duration::ZERO, Duration::from_nanos(1)))
                    .unwrap();

                metadata
                    .lock()
                    .await
                    .set::<(String, String)>(("".to_string(), "".to_string()))
                    .unwrap();

                playback
                    .lock()
                    .await
                    .set(UIPlaybackStatus {
                        playback_status: mpris::PlaybackStatus::Stopped,
                        can_playpause: false,
                        can_go_next: false,
                        can_go_previous: false,
                        can_loop: false,
                        can_shuffle: false,
                        shuffle: true,
                        loop_status: mpris::LoopStatus::Playlist,
                    })
                    .unwrap();
                return Err(anyhow::anyhow!("player quit"));
            }
            crate::player_info::MprisProgressEvent::Progress(prog) => {
                time.lock()
                    .await
                    .set::<(Duration, Duration)>((
                        prog.position,
                        prog.metadata.length().unwrap_or(Duration::ZERO),
                    ))
                    .unwrap();
                set_playback_status(&playback, &prog).await;
                let (song_name, artist_name) = (
                    match prog.metadata.title() {
                        Some(title) => title.to_string(),
                        None => "".to_string(),
                    },
                    match prog.metadata.artists() {
                        Some(artist) => artist
                            .first()
                            .map(|val| val.to_string())
                            .unwrap_or("".to_string()),
                        None => "".to_string(),
                    },
                );
                let mut new_trackid = prog
                    .metadata
                    .track_id()
                    .unwrap_or(TrackID::no_track())
                    .to_string();
                new_trackid.push_str(prog.metadata.title().unwrap_or(""));
                if new_trackid != track_id {
                    set_album_art(
                        prog.metadata.art_url(),
                        &config.default_album_art_url,
                        &album_art,
                        &visualizer_gradient,
                    )
                    .await;
                    track_id = new_trackid;
                }

                metadata
                    .lock()
                    .await
                    .set::<(String, String)>((song_name, artist_name))
                    .unwrap();
            }
        }
    }
    Ok(())
}

pub(crate) async fn action_task(
    player: MprisPlayer,
    seek_tx: UnboundedSender<Duration>,
    action_rx: Arc<Mutex<UnboundedReceiver<UIAction>>>,
) -> Result<()> {
    while let Some(action) = action_rx.lock().await.recv().await {
        match action {
            UIAction::Shuffle => {
                let res = player.set_shuffle(!player.get_shuffle().unwrap());
                if matches!(res, Err(DBusError::TransportError(_))) {
                    return Err(anyhow::anyhow!("failed to set shuffle"));
                }
            }
            UIAction::Previous => {
                if matches!(player.previous(), Err(DBusError::TransportError(_))) {
                    return Err(anyhow::anyhow!("failed to go to previous track"));
                }
            }
            UIAction::PlayPause => {
                if matches!(player.play_pause(), Err(DBusError::TransportError(_))) {
                    return Err(anyhow::anyhow!("failed to play/pause"));
                }
            }
            UIAction::Next => {
                if matches!(player.next(), Err(DBusError::TransportError(_))) {
                    return Err(anyhow::anyhow!("failed to go to next track"));
                }
            }
            UIAction::Loop => {
                if matches!(
                    player.set_loop(match player.get_loop().unwrap_or(mpris::LoopStatus::None) {
                        mpris::LoopStatus::None => mpris::LoopStatus::Track,
                        mpris::LoopStatus::Track => mpris::LoopStatus::Playlist,
                        mpris::LoopStatus::Playlist => mpris::LoopStatus::None,
                    }),
                    Err(DBusError::TransportError(_))
                ) {
                    return Err(anyhow::anyhow!("failed to set loop"));
                }
            }
            UIAction::SetPosition(pos) => {
                let tid = match player.get_current_track_id() {
                    Ok(tid) => tid,
                    Err(_) => {
                        return Err(anyhow::anyhow!("failed to get track id"));
                    }
                };
                let _ = player.set_position(tid.as_str(), pos);
                seek_tx.send(pos).expect("failed to refresh time");
            }
        }
    }
    Ok(())
}

pub(crate) async fn set_playback_status(
    playback: &Arc<Mutex<DynamicPropertyAny>>,
    prog: &crate::player_info::MprisProgress,
) {
    let old_playback_status = playback.lock().await;
    let playback_status = cast_dyn_any!(old_playback_status.get(), UIPlaybackStatus);
    let mut playback_status = playback_status.unwrap().clone();
    drop(old_playback_status);

    playback_status.playback_status = prog.playback_status;
    playback_status.shuffle = prog.shuffle;
    playback_status.loop_status = prog.loop_status;
    playback_status.can_go_next = prog.can_go_next;
    playback_status.can_go_previous = prog.can_go_prev;
    playback_status.can_loop = prog.can_loop;
    playback_status.can_shuffle = prog.can_shuffle;
    playback_status.can_playpause = prog.can_playpause;

    playback
        .lock()
        .await
        .set::<UIPlaybackStatus>(playback_status)
        .unwrap();
}

pub(crate) async fn wait_for_new_player_task(current_player_name: &str, preferred_player: &str) {
    let player_bus_name = if !preferred_player.is_empty() {
        preferred_player
    } else {
        current_player_name
    };

    let mut check_if_quit = false;
    if let Ok(pl) = MprisPlayer::find_new_player(&preferred_player) {
        if pl.bus_name_player_name_part() == player_bus_name {
            check_if_quit = true;
        }
    } else {
        return;
    }
    if check_if_quit {
        loop {
            if let Ok(pl) = MprisPlayer::find_new_player(&preferred_player) {
                if pl.bus_name_player_name_part() != player_bus_name {
                    return;
                }
            } else {
                return;
            }
            tokio::time::sleep(Duration::from_millis(CHECK_DELAY)).await;
        }
    }
    loop {
        //check if preferred player came back online
        if let Ok(pl) = MprisPlayer::find_new_player(&player_bus_name) {
            if pl.bus_name_player_name_part() == player_bus_name {
                return;
            }
        } else {
            return;
        }
        tokio::time::sleep(Duration::from_millis(CHECK_DELAY)).await;
    }
}

// TODO copy from script module
pub(crate) async fn set_album_art(
    art_url: Option<&str>,
    default_art_path: &str,
    album_art: &Arc<Mutex<DynamicPropertyAny>>,
    visualizer_gradient: &Arc<Mutex<DynamicPropertyAny>>,
) {
    let image = utils::get_album_art_from_url(art_url.unwrap_or_else(|| {
        log::debug!("no album art, using default");
        default_art_path
    }))
    .await
    .unwrap_or(
        utils::get_album_art_from_url(default_art_path)
            .await
            .unwrap_or(Vec::new()),
    );
    let gradient = visualizer::gradient_from_image_bytes(&image);
    album_art.lock().await.set(image).unwrap();
    visualizer_gradient.lock().await.set(gradient).unwrap();
}
