use std::collections::HashMap;

use dynisland_core::{
    abi::module::ActivityIdentifier,
    d_macro::{MultiWidgetConfig, OptDeserializeConfig},
};
use serde::Serialize;

#[derive(Debug, Serialize, Clone, MultiWidgetConfig, OptDeserializeConfig)]
#[serde(default)]
pub struct MusicConfig {
    pub(crate) preferred_player: String,
    pub(crate) default_album_art_url: String,
    pub(crate) scrolling_label_speed: f32,
    pub(crate) cava_visualizer_script: String,
    pub(crate) use_fallback_player: bool,
}
#[allow(clippy::derivable_impls)]
impl Default for MusicConfig {
    fn default() -> Self {
        Self {
            preferred_player: String::from(""),
            default_album_art_url: String::from(""),
            scrolling_label_speed: 30.0,
            cava_visualizer_script: String::from("echo 0,0,0,0,0,0"),
            use_fallback_player: true,
        }
    }
}

pub(crate) fn get_conf_idx(id: &ActivityIdentifier) -> usize {
    id.metadata()
        .additional_metadata("instance")
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
