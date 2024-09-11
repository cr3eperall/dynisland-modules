pub mod compact;
pub mod expanded;
pub mod minimal;
pub mod visualizer;

use std::time::Duration;

use compact::Compact;
use dynisland_core::{
    abi::{gdk, glib, gtk},
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
    dynamic_property::PropertyUpdate,
    graphics::activity_widget::{boxed_activity_mode::ActivityMode, ActivityWidget},
};
use expanded::Expanded;
use gdk::{gdk_pixbuf::Pixbuf, gio::MemoryInputStream};
use glib::{subclass::types::ObjectSubclassIsExt, Bytes};
use gtk::{prelude::*, GestureClick};
use minimal::Minimal;

pub enum UIAction {
    Shuffle,
    Previous,
    PlayPause,
    Next,
    Loop,
    SetPosition(Duration),
}
#[derive(Debug, Clone)]
pub struct UIPlaybackStatus {
    pub playback_status: mpris::PlaybackStatus,
    pub can_playpause: bool,
    pub can_go_next: bool,
    pub can_go_previous: bool,
    pub can_loop: bool,
    pub can_shuffle: bool,
    pub shuffle: bool,
    pub loop_status: mpris::LoopStatus,
}
impl Default for UIPlaybackStatus {
    fn default() -> Self {
        UIPlaybackStatus {
            playback_status: mpris::PlaybackStatus::Stopped,
            can_playpause: false,
            can_go_next: false,
            can_go_previous: false,
            can_loop: false,
            can_shuffle: false,
            shuffle: false,
            loop_status: mpris::LoopStatus::None,
        }
    }
}

pub fn get_activity(
    prop_send: tokio::sync::mpsc::UnboundedSender<PropertyUpdate>,
    module: &str,
    name: &str,
    window: &str,
    idx: usize,
) -> DynamicActivity {
    let mut activity = DynamicActivity::new_with_metadata(
        prop_send,
        module,
        &(name.to_string() + "-" + &idx.to_string()),
        Some(window),
        vec![("instance".to_string(), idx.to_string())],
    );

    let activity_widget = activity.get_activity_widget();
    activity_widget.add_css_class(name);

    //get widgets
    let minimal = Minimal::new();
    let compact = Compact::new();
    let expanded = Expanded::new();

    //load widgets in the activity widget
    activity_widget.set_minimal_mode_widget(minimal.clone());
    activity_widget.set_compact_mode_widget(compact.clone());
    activity_widget.set_expanded_mode_widget(expanded.clone());

    setup_music_metadata_prop(&mut activity, &compact, &expanded);

    setup_album_art_prop(&mut activity, &minimal, &compact, &expanded);

    setup_visualizer_data_prop(&mut activity, &activity_widget);

    setup_visualizer_gradient_prop(&mut activity);

    setup_music_time_prop(&mut activity, &activity_widget);

    setup_playback_status_prop(&mut activity, &expanded);

    setup_scrolling_label_speed_prop(&mut activity, &compact, &expanded);

    register_mode_gestures(activity_widget);

    activity
}

fn register_mode_gestures(activity_widget: ActivityWidget) {
    let press_gesture = gtk::GestureClick::new();
    press_gesture.set_button(gdk::BUTTON_PRIMARY);

    press_gesture.connect_released(move |gest, _, x, y| {
        let aw = gest.widget().downcast::<ActivityWidget>().unwrap();
        if x < 0.0
            || y < 0.0
            || x > aw.size(gtk::Orientation::Horizontal).into()
            || y > aw.size(gtk::Orientation::Vertical).into()
        {
            return;
        }
        match aw.mode() {
            ActivityMode::Compact => {
                aw.set_mode(ActivityMode::Expanded);
            }
            ActivityMode::Minimal | ActivityMode::Expanded | ActivityMode::Overlay => {}
        }
    });

    activity_widget.add_controller(press_gesture);

    let release_gesture = GestureClick::new();
    release_gesture.set_button(gdk::BUTTON_SECONDARY);
    release_gesture.connect_released(move |gest, _, x, y| {
        let aw = gest.widget().downcast::<ActivityWidget>().unwrap();
        if x < 0.0
            || y < 0.0
            || x > aw.size(gtk::Orientation::Horizontal).into()
            || y > aw.size(gtk::Orientation::Vertical).into()
        {
            return;
        }
        match aw.mode() {
            ActivityMode::Compact => {
                aw.set_mode(ActivityMode::Minimal);
            }
            ActivityMode::Expanded => {
                aw.set_mode(ActivityMode::Compact);
            }
            ActivityMode::Minimal | ActivityMode::Overlay => {}
        }
    });
    activity_widget.add_controller(release_gesture);
}

fn setup_playback_status_prop(activity: &mut DynamicActivity, expanded: &Expanded) {
    activity
        .add_dynamic_property("playback-status", UIPlaybackStatus::default())
        .unwrap();
    {
        let shuffle = expanded.imp().shuffle.clone();
        let previous = expanded.imp().previous.clone();
        let play_pause = expanded.imp().play_pause.clone();
        let next = expanded.imp().next.clone();
        let repeat = expanded.imp().repeat.clone();

        activity
            .subscribe_to_property("playback-status", move |new_value| {
                let playback_status = cast_dyn_any!(new_value, UIPlaybackStatus).unwrap();
                match playback_status.can_shuffle {
                    true => {
                        match playback_status.shuffle {
                            true => {
                                shuffle.set_icon_name("media-playlist-shuffle-symbolic");
                            }
                            false => {
                                shuffle.set_icon_name("media-playlist-consecutive-symbolic");
                            }
                        }
                        shuffle.set_sensitive(true);
                    }
                    false => {
                        shuffle.set_icon_name("media-playlist-shuffle-symbolic");
                        shuffle.set_sensitive(false);
                    }
                }

                match playback_status.can_go_previous {
                    true => {
                        previous.set_sensitive(true);
                    }
                    false => {
                        previous.set_sensitive(false);
                    }
                }
                match playback_status.can_playpause {
                    true => {
                        match playback_status.playback_status {
                            mpris::PlaybackStatus::Playing => {
                                play_pause.set_icon_name("media-playback-pause-symbolic");
                            }
                            mpris::PlaybackStatus::Paused => {
                                play_pause.set_icon_name("media-playback-start-symbolic");
                            }
                            mpris::PlaybackStatus::Stopped => {
                                play_pause.set_icon_name("media-playback-stop-symbolic");
                            }
                        }
                        play_pause.set_sensitive(true);
                    }
                    false => {
                        play_pause.set_icon_name("media-playback-stop-symbolic");
                        play_pause.set_sensitive(false);
                    }
                }

                match playback_status.can_go_next {
                    true => {
                        next.set_sensitive(true);
                    }
                    false => {
                        next.set_sensitive(false);
                    }
                }
                match playback_status.can_loop {
                    true => {
                        match playback_status.loop_status {
                            mpris::LoopStatus::None => {
                                repeat.set_icon_name("mail-forward");
                            }
                            mpris::LoopStatus::Track => {
                                repeat.set_icon_name("media-playlist-repeat-song-symbolic");
                            }
                            mpris::LoopStatus::Playlist => {
                                repeat.set_icon_name("media-playlist-repeat-symbolic");
                            }
                        }
                        repeat.set_sensitive(true);
                    }
                    false => {
                        repeat.set_icon_name("media-playlist-repeat-symbolic");
                        repeat.set_sensitive(false);
                    }
                }
            })
            .unwrap();
    }
}

fn setup_music_time_prop(activity: &mut DynamicActivity, activity_widget: &ActivityWidget) {
    activity
        .add_dynamic_property("music-time", (Duration::ZERO, Duration::ZERO))
        .unwrap();
    {
        let expanded = activity_widget
            .expanded_mode_widget()
            .unwrap()
            .downcast::<Expanded>()
            .unwrap();
        let elapsed = expanded.imp().elapsed_time.clone();
        let progress_bar = expanded.imp().progress_bar.clone();
        let remaining = expanded.imp().remaining_time.clone();
        let aw = activity_widget.clone();
        activity
            .subscribe_to_property("music-time", move |new_value| {
                let (mut current_time, mut total_duration) =
                    cast_dyn_any!(new_value, (Duration, Duration)).unwrap();
                if let ActivityMode::Expanded = aw.mode() {
                    progress_bar.set_range(0.0, total_duration.as_millis() as f64);

                    if !progress_bar.has_css_class("dragging") {
                        progress_bar.set_value(current_time.as_millis() as f64);
                    }
                    current_time = Duration::from_secs(current_time.as_secs());
                    total_duration = Duration::from_secs(total_duration.as_secs());
                    elapsed.set_label(&format!(
                        "{:02}:{:02}",
                        current_time.as_secs() / 60,
                        current_time.as_secs() % 60
                    ));
                    let remaining_time = total_duration.saturating_sub(current_time);
                    remaining.set_label(&format!(
                        "-{:02}:{:02}",
                        remaining_time.as_secs() / 60,
                        remaining_time.as_secs() % 60
                    ));
                }
            })
            .unwrap();
    }
}

fn setup_visualizer_gradient_prop(activity: &mut DynamicActivity) {
    let gradient_css_provider = gtk::CssProvider::new();
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().unwrap(),
        &gradient_css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
    activity
        .add_dynamic_property("visualizer-gradient", [[[0_u8; 3]; 6]; 3])
        .unwrap();
    {
        let css_class = activity.get_activity_widget().name();
        activity
            .subscribe_to_property("visualizer-gradient", move |new_value| {
                let data = cast_dyn_any!(new_value, [[[u8; 3]; 6]; 3]).unwrap();
                gradient_css_provider
                    .load_from_string(&visualizer::get_gradient_css(&css_class, data))
            })
            .unwrap();
    }
}

fn setup_visualizer_data_prop(activity: &mut DynamicActivity, activity_widget: &ActivityWidget) {
    let bar_height_css_provider = gtk::CssProvider::new();
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().unwrap(),
        &bar_height_css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER,
    );
    activity
        .add_dynamic_property("visualizer-data", [0_u8; 6])
        .unwrap();
    {
        let aw = activity_widget.clone();
        let css_name = aw.name();
        activity
            .subscribe_to_property("visualizer-data", move |new_value| {
                let data = cast_dyn_any!(new_value, [u8; 6]).unwrap();
                bar_height_css_provider.load_from_string(&visualizer::get_bar_css(
                    &css_name,
                    data,
                    32,
                    30,
                    60,
                    aw.mode(),
                ));
            })
            .unwrap();
    }
}

fn setup_album_art_prop(
    activity: &mut DynamicActivity,
    minimal: &Minimal,
    compact: &Compact,
    expanded: &Expanded,
) {
    let empty: Vec<u8> = Vec::new();
    activity.add_dynamic_property("album-art", empty).unwrap();
    {
        let expanded_album_art = expanded.imp().image.clone();
        let compact_album_art = compact.imp().image.clone();
        let minimal_album_art = minimal.imp().image.clone();

        activity
            .subscribe_to_property("album-art", move |new_value| {
                let buf = cast_dyn_any!(new_value, Vec<u8>).unwrap();
                let data = buf.as_slice();
                let data = Bytes::from(data);
                let mut pixbuf = Pixbuf::from_stream(
                    &MemoryInputStream::from_bytes(&data),
                    None::<&gtk::gio::Cancellable>,
                )
                .ok();
                if pixbuf.is_none() {
                    pixbuf = Pixbuf::new(gdk::gdk_pixbuf::Colorspace::Rgb, true, 8, 10, 10);
                }
                let texture = gdk::Texture::for_pixbuf(&pixbuf.unwrap());
                expanded_album_art.set_paintable(Some(&texture));
                compact_album_art.set_paintable(Some(&texture));
                minimal_album_art.set_paintable(Some(&texture));
            })
            .unwrap();
    }
}

fn setup_music_metadata_prop(
    activity: &mut DynamicActivity,
    compact: &Compact,
    expanded: &Expanded,
) {
    activity
        .add_dynamic_property("music-metadata", (String::new(), String::new()))
        .unwrap();
    {
        let song_name_widget = expanded.imp().song_name.clone();
        let artist_name_widget = expanded.imp().artist_name.clone();
        let compact_song_name_widget = compact.imp().song_name.clone();
        activity
            .subscribe_to_property("music-metadata", move |new_value| {
                let (song_name, artist_name) = cast_dyn_any!(new_value, (String, String)).unwrap();
                song_name_widget.set_text(song_name.as_str());
                artist_name_widget.set_label(artist_name);
                compact_song_name_widget.set_text(song_name.as_str());
            })
            .unwrap();
    }
}

fn setup_scrolling_label_speed_prop(
    activity: &mut DynamicActivity,
    compact: &Compact,
    expanded: &Expanded,
) {
    activity
        .add_dynamic_property("scrolling-label-speed", 30.0_f32)
        .unwrap();
    {
        let expanded_label = expanded.imp().song_name.clone();
        let compact_label = compact.imp().song_name.clone();
        activity
            .subscribe_to_property("scrolling-label-speed", move |new_value| {
                let data = cast_dyn_any!(new_value, f32).unwrap();
                expanded_label.set_config_scroll_speed(*data);
                compact_label.set_config_scroll_speed(*data);
            })
            .unwrap();
    }
}
