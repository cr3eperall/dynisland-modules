use std::rc::Rc;

use dynisland_core::{
    abi::gtk,
    d_macro::{MultiWidgetConfig, OptDeserializeConfig},
    dynamic_activity::DynamicActivity,
};
use gtk::{prelude::*, subclass::prelude::*};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::widget::{battery::Battery, compact::Compact, expanded::Expanded, minimal::Minimal};

#[derive(Debug, Serialize, MultiWidgetConfig, OptDeserializeConfig, Clone)]
#[serde(default)]
pub struct PowerConfig {
    pub(crate) battery: String,
    pub(crate) name: String,
    pub(crate) hide_if_missing: bool,
    pub(crate) percentage_pango_markup: String,
    pub(crate) charging_color: String,
    pub(crate) normal_color: String,
    pub(crate) low_color: String,
    pub(crate) background_color: String,
    pub(crate) max_duration_secs: u32,
    pub(crate) draw_bars: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for PowerConfig {
    fn default() -> Self {
        Self {
            battery: "".to_string(),
            name: "".to_string(),
            hide_if_missing: true,
            percentage_pango_markup:
                "<span font_desc=\"Monospace Bold 11\" foreground=\"#FFFFFF00\">{}</span>"
                    .to_string(),
            charging_color: "#2EE81AFF".to_string(),
            normal_color: "white".to_string(),
            low_color: "#FDC400FF".to_string(),
            background_color: "#E6E6E699".to_string(),
            max_duration_secs: 36000,
            draw_bars: true,
        }
    }
}

impl PowerConfig {
    pub fn apply_to(&self, dyn_act: &Rc<Mutex<DynamicActivity>>) {
        let widget = dyn_act.blocking_lock().get_activity_widget();
        let minimal = widget
            .minimal_mode_widget()
            .unwrap()
            .downcast::<Minimal>()
            .unwrap();
        let compact = widget
            .compact_mode_widget()
            .unwrap()
            .downcast::<Compact>()
            .unwrap();
        let expanded = widget
            .expanded_mode_widget()
            .unwrap()
            .downcast::<Expanded>()
            .unwrap();

        let tooltip = (!self.name.is_empty()).then(|| self.name.as_str());

        minimal.set_charging_color(self.charging_color.clone());
        minimal.set_normal_color(self.normal_color.clone());
        minimal.set_low_battery_color(self.low_color.clone());
        minimal.set_tooltip_text(tooltip);
        let minimal_battery = minimal.imp().battery.clone().downcast::<Battery>().unwrap();
        minimal_battery.set_background_color(self.background_color.clone());
        minimal_battery.set_percentage_markup(self.percentage_pango_markup.clone());
        compact.set_charging_color(self.charging_color.clone());
        compact.set_normal_color(self.normal_color.clone());
        compact.set_low_battery_color(self.low_color.clone());
        compact.set_tooltip_text(tooltip);
        let compact_battery = compact.imp().battery.clone().downcast::<Battery>().unwrap();
        compact_battery.set_background_color(self.background_color.clone());
        compact_battery.set_percentage_markup(self.percentage_pango_markup.clone());
        expanded.imp().graph.set_draw_bars(self.draw_bars);
        expanded
            .imp()
            .graph
            .set_max_duration_secs(self.max_duration_secs);
    }
}
