use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// TODO cleanup

#[derive(Debug, Serialize, Clone)]
#[serde(default)]
pub struct ExampleConfigMain {
    pub(crate) int: i32,
    pub(crate) string: String,
    pub(crate) vec: Vec<String>,
    pub(crate) duration: u64,
    pub(crate) windows: HashMap<String, Vec<ExampleConfig>>,
}

impl Default for ExampleConfigMain {
    fn default() -> Self {
        let mut map = HashMap::new();
        map.insert("".to_string(), vec![ExampleConfig::default()]);
        Self {
            int: 0,
            string: String::from("Example1"),
            vec: vec![String::from("Example2"), String::from("Example3")],
            duration: 400,
            windows: map,
        }
    }
}

impl ExampleConfigMain {
    pub fn default_conf(&self) -> ExampleConfig {
        ExampleConfig {
            int: self.int,
            string: self.string.clone(),
            vec: self.vec.clone(),
            duration: self.duration,
        }
    }
    pub fn get_for_window(&self, window: &str) -> Vec<ExampleConfig> {
        match self.windows.get(window) {
            Some(conf) => conf.clone(),
            None => vec![self.default_conf()],
        }
    }
}

#[derive(Debug, Serialize, Clone)]
#[serde(default)]
pub struct ExampleConfig {
    pub(crate) int: i32,
    pub(crate) string: String,
    pub(crate) vec: Vec<String>,
    pub(crate) duration: u64,
}

impl Default for ExampleConfig {
    fn default() -> Self {
        Self {
            int: 0,
            string: String::from("Example1"),
            vec: vec![String::from("Example2"), String::from("Example3")],
            duration: 400,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeExampleConfigMain {
    int: i32,
    string: String,
    vec: Vec<String>,
    duration: u64,
    windows: HashMap<String, Vec<DeExampleConfig>>,
}  

impl Default for DeExampleConfigMain {
    fn default() -> Self {
        let map = HashMap::new();
        Self {
            int: 0,
            string: String::from("Example1"),
            vec: vec![String::from("Example2"), String::from("Example3")],
            duration: 400,
            windows: map,
        }
    }
}

impl DeExampleConfigMain {
    pub fn into_main_config(self) -> ExampleConfigMain {
        let mut windows = HashMap::new();
        for (window_name, opt_conf_vec) in self.windows {
            let mut conf_vec = Vec::new();
            for opt_conf in opt_conf_vec {
                let conf = ExampleConfig {
                    int: opt_conf.int.unwrap_or(self.int),
                    string: opt_conf.string.unwrap_or(self.string.clone()),
                    vec: opt_conf.vec.unwrap_or(self.vec.clone()),
                    duration: opt_conf.duration.unwrap_or(self.duration),
                };
                conf_vec.push(conf);
            }
            windows.insert(window_name, conf_vec);
        }
        if windows.is_empty() {
            windows.insert(
                "".to_string(),
                vec![ExampleConfig {
                    int: self.int,
                    string: self.string.clone(),
                    vec: self.vec.clone(),
                    duration: self.duration,
                }],
            );
        }
        ExampleConfigMain {
            int: self.int,
            string: self.string,
            vec: self.vec,
            duration: self.duration,
            windows,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
#[serde(default)]
pub struct DeExampleConfig {
    number_of_widgets: Option<u32>,
    int: Option<i32>,
    string: Option<String>,
    vec: Option<Vec<String>>,
    duration: Option<u64>,
}
