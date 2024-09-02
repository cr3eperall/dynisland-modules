use serde::{Deserialize, Serialize};

// #[derive(Debug, Deserialize, Clone)]
// #[serde(default)]
// pub struct MusicConfigMain {
//     pub(crate) preferred_player: String,
//     pub(crate) default_album_art_url: String,
//     pub(crate) scrolling_label_speed: f32,
//     pub(crate) cava_visualizer_script: String,
//     pub(crate) windows: HashMap<String, Vec<MusicConfig>>,
// }

// impl Default for MusicConfigMain {
//     fn default() -> Self {
//         let map = HashMap::new();
//         Self {
//             preferred_player: String::from(""),
//             default_album_art_url: String::from(""),
//             scrolling_label_speed: 30.0,
//             cava_visualizer_script: String::from("echo 0,0,0,0,0,0"),
//             windows: map,
//         }
//     }
// }

// impl MusicConfigMain {
//     pub fn default_conf(&self) -> MusicConfig {
//         MusicConfig {
//             preferred_player: self.preferred_player.clone(),
//             default_album_art_url: self.default_album_art_url.clone(),
//             scrolling_label_speed: self.scrolling_label_speed,
//             cava_visualizer_script: self.cava_visualizer_script.clone(),
//         }
//     }
//     pub fn get_for_window(&self, window: &str) -> Vec<MusicConfig> {
//         match self.windows.get(window) {
//             Some(conf) => conf.clone(),
//             None => vec![self.default_conf()],
//         }
//     }
// }

#[derive(Debug, Serialize, Deserialize, Clone)]
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

// #[derive(Debug, Serialize, Clone)]
// #[serde(default)]
// pub struct MusicConfigMainOptional {
//     pub(crate) preferred_player: String,
//     pub(crate) default_album_art_url: String,
//     pub(crate) scrolling_label_speed: f32,
//     pub(crate) cava_visualizer_script: String,
//     windows: HashMap<String, Vec<MusicConfigOptional>>,
// }

// impl Default for MusicConfigMainOptional {
//     fn default() -> Self {
//         let map = HashMap::new();
//         Self {
//             preferred_player: String::from(""),
//             default_album_art_url: String::from(""),
//             scrolling_label_speed: 30.0,
//             cava_visualizer_script: String::from("echo 0,0,0,0,0,0"),
//             windows: map,
//         }
//     }
// }

// impl MusicConfigMainOptional {
//     pub fn into_main_config(self) -> MusicConfigMain {
//         let mut windows = HashMap::new();
//         for (window_name, opt_conf_vec) in self.windows {
//             let mut conf_vec = Vec::new();
//             for opt_conf in opt_conf_vec {
//                 let conf = MusicConfig {
//                     preferred_player: opt_conf.preferred_player.unwrap_or(self.preferred_player.clone()),
//                     default_album_art_url: opt_conf.default_album_art_url.unwrap_or(self.default_album_art_url.clone()),
//                     scrolling_label_speed: opt_conf.scrolling_label_speed.unwrap_or(self.scrolling_label_speed),
//                     cava_visualizer_script: opt_conf.cava_visualizer_script.unwrap_or(self.cava_visualizer_script.clone()),
//                 };
//                 conf_vec.push(conf);
//             }
//             windows.insert(window_name, conf_vec);
//         }
//         if windows.is_empty() {
//             windows.insert(String::from("default"), vec![MusicConfig{
//                 preferred_player: self.preferred_player.clone(),
//                 default_album_art_url: self.default_album_art_url.clone(),
//                 scrolling_label_speed: self.scrolling_label_speed,
//                 cava_visualizer_script: self.cava_visualizer_script.clone(),
//             }]);
//         }
//         MusicConfigMain {
//             preferred_player: self.preferred_player,
//             default_album_art_url: self.default_album_art_url,
//             scrolling_label_speed: self.scrolling_label_speed,
//             cava_visualizer_script: self.cava_visualizer_script,
//             windows,
//         }
//     }
// }

// #[derive(Debug, Serialize, Clone, Default)]
// #[serde(default)]
// pub struct MusicConfigOptional {
//     pub(crate) preferred_player: Option<String>,
//     pub(crate) default_album_art_url: Option<String>,
//     pub(crate) scrolling_label_speed: Option<f32>,
//     pub(crate) cava_visualizer_script: Option<String>,
// }
