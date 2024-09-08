use std::collections::HashMap;

use dynisland_core::abi::log;
use serde::{Deserialize, Serialize};

// TODO cleanup

#[derive(Debug, Serialize, Clone)]
pub struct ScriptConfigMain {
    pub(crate) scrolling: bool,
    pub(crate) scrolling_speed: f32,
    pub(crate) max_width: i32,
    pub(crate) minimal_image: String,
    pub(crate) windows: std::collections::HashMap<String, Vec<ScriptConfig>>,
}

impl Default for ScriptConfigMain {
    fn default() -> Self {
        let mut map = std::collections::HashMap::new();
        map.insert("".to_string(), vec![ScriptConfig::default()]);
        Self {
            scrolling: true,
            scrolling_speed: 30.0,
            max_width: 300,
            minimal_image: String::from("image-missing-symbolic"),
            windows: map,
        }
    }
}

impl ScriptConfigMain {
    pub fn default_conf(&self) -> ScriptConfig {
        ScriptConfig {
            scrolling: self.scrolling,
            scrolling_speed: self.scrolling_speed,
            max_width: self.max_width,
            minimal_image: self.minimal_image.clone(),
            exec: "".to_string(),
        }
    }
    pub fn get_for_window(&self, window: &str) -> Vec<ScriptConfig> {
        match self.windows.get(window) {
            Some(conf) => conf.clone(),
            None => vec![self.default_conf()],
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct ScriptConfig {
    #[serde(skip_serializing)]
    pub(crate) scrolling: bool,
    #[serde(skip_serializing)]
    pub(crate) scrolling_speed: f32,
    /// if scrolling, it's in pixels, if not, it's in chars
    #[serde(skip_serializing)]
    pub(crate) max_width: i32,
    #[serde(skip_serializing)]
    pub(crate) minimal_image: String,
    pub(crate) exec: String,
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

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DeScriptConfigMain {
    scrolling: bool,
    scrolling_speed: f32,
    max_width: i32,
    minimal_image: String,
    windows: HashMap<String, Vec<DeScriptConfig>>,
}

impl Default for DeScriptConfigMain {
    fn default() -> Self {
        let map = HashMap::new();
        Self {
            scrolling: true,
            scrolling_speed: 30.0,
            max_width: 300,
            minimal_image: String::from("image-missing-symbolic"),
            windows: map,
        }
    }
}

impl DeScriptConfigMain {
    pub fn into_main_config(self) -> ScriptConfigMain {
        let mut windows = HashMap::new();
        for (name, opt_vec_conf) in self.windows {
            let mut vec_conf = Vec::new();
            for opt_conf in opt_vec_conf {
                let conf = ScriptConfig {
                    scrolling: opt_conf.scrolling.unwrap_or(self.scrolling),
                    scrolling_speed: opt_conf.scrolling_speed.unwrap_or(self.scrolling_speed),
                    max_width: opt_conf.max_width.unwrap_or(self.max_width),
                    minimal_image: opt_conf.minimal_image.unwrap_or(self.minimal_image.clone()),
                    exec: opt_conf.exec,
                };
                vec_conf.push(conf);
            }
            windows.insert(name, vec_conf);
        }
        if windows.is_empty() {
            log::warn!("No window found for ScriptModule, see wiki for more information or update your config file from `dynisland default-config`");
            let script = ScriptConfig {
                exec: "echo \"update your config file: see wiki\"".to_string(),
                minimal_image: self.minimal_image.clone(),
                scrolling: self.scrolling,
                scrolling_speed: self.scrolling_speed,
                max_width: self.max_width,
            };
            windows.insert("".to_string(), vec![script]);
        }
        ScriptConfigMain {
            scrolling: self.scrolling,
            scrolling_speed: self.scrolling_speed,
            max_width: self.max_width,
            minimal_image: self.minimal_image,
            windows,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DeScriptConfig {
    #[serde(default)]
    scrolling: Option<bool>,
    #[serde(default)]
    scrolling_speed: Option<f32>,
    #[serde(default)]
    max_width: Option<i32>,
    #[serde(default)]
    minimal_image: Option<String>,
    exec: String,
}
