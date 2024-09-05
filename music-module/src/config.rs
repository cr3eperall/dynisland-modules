use std::collections::HashMap;

use dynisland_core::abi::module::ActivityIdentifier;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
#[serde(default)]
pub struct MusicConfigMain {
    pub(crate) preferred_player: String,
    pub(crate) default_album_art_url: String,
    pub(crate) scrolling_label_speed: f32,
    pub(crate) cava_visualizer_script: String,
    pub(crate) windows: HashMap<String, Vec<MusicConfig>>,
}

impl Default for MusicConfigMain {
    fn default() -> Self {
        let mut map = HashMap::new();
        map.insert("".to_string(), vec![MusicConfig::default()]);
        Self {
            preferred_player: String::from(""),
            default_album_art_url: String::from(""),
            scrolling_label_speed: 30.0,
            cava_visualizer_script: String::from("echo 0,0,0,0,0,0"),
            windows: map,
        }
    }
}

impl MusicConfigMain {
    pub fn default_conf(&self) -> MusicConfig {
        MusicConfig {
            preferred_player: self.preferred_player.clone(),
            default_album_art_url: self.default_album_art_url.clone(),
            scrolling_label_speed: self.scrolling_label_speed,
            cava_visualizer_script: self.cava_visualizer_script.clone(),
        }
    }
    pub fn get_for_window(&self, window: &str) -> Vec<MusicConfig> {
        match self.windows.get(window) {
            Some(conf) => conf.clone(),
            None => vec![self.default_conf()],
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(default)]
pub struct MusicConfig {
    pub(crate) preferred_player: String,
    pub(crate) default_album_art_url: String,
    pub(crate) scrolling_label_speed: f32,
    pub(crate) cava_visualizer_script: String,
}
#[allow(clippy::derivable_impls)]
impl Default for MusicConfig {
    fn default() -> Self {
        Self {
            preferred_player: String::from(""),
            default_album_art_url: String::from(""),
            scrolling_label_speed: 30.0,
            cava_visualizer_script: String::from("echo 0,0,0,0,0,0"),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DeMusicConfigMain {
    preferred_player: String,
    default_album_art_url: String,
    scrolling_label_speed: f32,
    cava_visualizer_script: String,
    windows: HashMap<String, Vec<DeMusicConfig>>,
}

impl Default for DeMusicConfigMain {
    fn default() -> Self {
        let map = HashMap::new();
        Self {
            preferred_player: String::from(""),
            default_album_art_url: String::from(""),
            scrolling_label_speed: 30.0,
            cava_visualizer_script: String::from("echo 0,0,0,0,0,0"),
            windows: map,
        }
    }
}

impl DeMusicConfigMain {
    pub fn into_main_config(self) -> MusicConfigMain {
        let mut windows = HashMap::new();
        for (window_name, opt_conf_vec) in self.windows {
            let mut conf_vec = Vec::new();
            for opt_conf in opt_conf_vec {
                let conf = MusicConfig {
                    preferred_player: opt_conf
                        .preferred_player
                        .unwrap_or(self.preferred_player.clone()),
                    default_album_art_url: opt_conf
                        .default_album_art_url
                        .unwrap_or(self.default_album_art_url.clone()),
                    scrolling_label_speed: opt_conf
                        .scrolling_label_speed
                        .unwrap_or(self.scrolling_label_speed),
                    cava_visualizer_script: opt_conf
                        .cava_visualizer_script
                        .unwrap_or(self.cava_visualizer_script.clone()),
                };
                conf_vec.push(conf);
            }
            windows.insert(window_name, conf_vec);
        }
        if windows.is_empty() {
            windows.insert(
                String::from(""),
                vec![MusicConfig {
                    preferred_player: self.preferred_player.clone(),
                    default_album_art_url: self.default_album_art_url.clone(),
                    scrolling_label_speed: self.scrolling_label_speed,
                    cava_visualizer_script: self.cava_visualizer_script.clone(),
                }],
            );
        }
        MusicConfigMain {
            preferred_player: self.preferred_player,
            default_album_art_url: self.default_album_art_url,
            scrolling_label_speed: self.scrolling_label_speed,
            cava_visualizer_script: self.cava_visualizer_script,
            windows,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct DeMusicConfig {
    preferred_player: Option<String>,
    default_album_art_url: Option<String>,
    scrolling_label_speed: Option<f32>,
    cava_visualizer_script: Option<String>,
}

pub fn get_conf_idx(id: &ActivityIdentifier) -> usize {
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
