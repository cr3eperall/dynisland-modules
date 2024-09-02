use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// TODO cleanup

#[derive(Debug, Serialize, Clone)]
#[serde(default)]
pub struct ClockConfigMain {
    pub(crate) format_24h: bool,
    pub(crate) hour_hand_color: String,
    pub(crate) minute_hand_color: String,
    pub(crate) tick_color: String,
    pub(crate) circle_color: String,
    pub(crate) windows: HashMap<String, Vec<ClockConfig>>,
}

impl Default for ClockConfigMain {
    fn default() -> Self {
        let mut map = HashMap::new();
        map.insert("".to_string(), vec![ClockConfig::default()]);
        Self {
            format_24h: true,
            hour_hand_color: String::from("white"),
            minute_hand_color: String::from("white"),
            circle_color: String::from("lightgray"),
            tick_color: String::from("lightgray"),
            windows: map,
        }
    }
}

impl ClockConfigMain {
    pub fn default_conf(&self) -> ClockConfig {
        ClockConfig {
            format_24h: self.format_24h,
            hour_hand_color: self.hour_hand_color.clone(),
            minute_hand_color: self.minute_hand_color.clone(),
            tick_color: self.tick_color.clone(),
            circle_color: self.circle_color.clone(),
        }
    }
    pub fn get_for_window(&self, window: &str) -> Vec<ClockConfig> {
        match self.windows.get(window) {
            Some(conf) => conf.clone(),
            None => vec![self.default_conf()],
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(default)]
pub struct ClockConfig {
    pub(crate) format_24h: bool,
    pub(crate) hour_hand_color: String,
    pub(crate) minute_hand_color: String,
    pub(crate) tick_color: String,
    pub(crate) circle_color: String,
}

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

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct DeClockConfigMain {
    format_24h: bool,
    hour_hand_color: String,
    minute_hand_color: String,
    tick_color: String,
    circle_color: String,
    windows: HashMap<String, Vec<DeClockConfig>>,
}

impl Default for DeClockConfigMain {
    fn default() -> Self {
        Self {
            format_24h: true,
            hour_hand_color: String::from("white"),
            minute_hand_color: String::from("white"),
            circle_color: String::from("lightgray"),
            tick_color: String::from("lightgray"),
            windows: HashMap::new(),
        }
    }
}
impl DeClockConfigMain {
    pub fn into_main_config(self) -> ClockConfigMain {
        let mut windows = HashMap::new();
        for (window_name, opt_conf_vec) in self.windows {
            let mut conf_vec = Vec::new();
            for opt_conf in opt_conf_vec {
                let conf = ClockConfig {
                    format_24h: opt_conf.format_24h.unwrap_or(self.format_24h),
                    hour_hand_color: opt_conf
                        .hour_hand_color
                        .unwrap_or(self.hour_hand_color.clone()),
                    minute_hand_color: opt_conf
                        .minute_hand_color
                        .unwrap_or(self.minute_hand_color.clone()),
                    tick_color: opt_conf.tick_color.unwrap_or(self.tick_color.clone()),
                    circle_color: opt_conf.circle_color.unwrap_or(self.circle_color.clone()),
                };
                conf_vec.push(conf);
            }
            windows.insert(window_name, conf_vec);
        }
        if windows.is_empty() {
            windows.insert(
                "".to_string(),
                vec![ClockConfig {
                    format_24h: self.format_24h,
                    hour_hand_color: self.hour_hand_color.clone(),
                    minute_hand_color: self.minute_hand_color.clone(),
                    tick_color: self.tick_color.clone(),
                    circle_color: self.circle_color.clone(),
                }],
            );
        }
        ClockConfigMain {
            format_24h: self.format_24h,
            hour_hand_color: self.hour_hand_color,
            minute_hand_color: self.minute_hand_color,
            tick_color: self.tick_color,
            circle_color: self.circle_color,
            windows,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct DeClockConfig {
    format_24h: Option<bool>,
    hour_hand_color: Option<String>,
    minute_hand_color: Option<String>,
    tick_color: Option<String>,
    circle_color: Option<String>,
}
