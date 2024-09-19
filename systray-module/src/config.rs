use dynisland_core::d_macro::{MultiWidgetConfig, OptDeserializeConfig};
use serde::Serialize;

// TODO add option for using the classic gtk menu popup
#[derive(Debug, Serialize, MultiWidgetConfig, OptDeserializeConfig, Clone)]
#[serde(default)]
pub struct SystrayConfig {
    /// supported values: "max", "2-step" or "current"
    pub(crate) menu_height_mode: String,
}

#[allow(clippy::derivable_impls)]
impl Default for SystrayConfig {
    fn default() -> Self {
        Self {
            menu_height_mode: String::from("2-step"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum MenuHeightMode {
    #[default]
    TwoStep,
    Max,
    Current,
}

impl From<&str> for MenuHeightMode {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "max" => MenuHeightMode::Max,
            "current" => MenuHeightMode::Current,
            "2-step" => MenuHeightMode::TwoStep,
            _ => MenuHeightMode::TwoStep,
        }
    }
}
