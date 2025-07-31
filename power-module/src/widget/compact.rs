use std::cell::RefCell;

use dynisland_core::{
    abi::{glib, gtk},
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
};
use glib::{
    prelude::ObjectExt,
    subclass::{
        object::{DerivedObjectProperties, ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
        InitializingObject,
    },
    types::StaticTypeExt,
    Object, Properties,
};
use gtk::{
    prelude::WidgetExt,
    subclass::widget::{
        CompositeTemplateClass, CompositeTemplateDisposeExt, CompositeTemplateInitializingExt,
        WidgetClassExt, WidgetImpl,
    },
    BinLayout, CompositeTemplate, TemplateChild,
};

use super::battery::Battery;
use crate::upower;

glib::wrapper! {
    pub struct Compact(ObjectSubclass<CompactPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate, Properties)]
#[properties(wrapper_type = Compact)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/powerModule/compact.ui")]
pub struct CompactPriv {
    #[template_child]
    pub battery: TemplateChild<Battery>,
    #[template_child]
    pub label: TemplateChild<gtk::Label>,
    #[property(get, set)]
    pub low_battery_color: RefCell<String>,
    #[property(get, set)]
    pub charging_color: RefCell<String>,
    #[property(get, set)]
    pub normal_color: RefCell<String>,
    //TODO add low battery treshold %
}

impl Default for CompactPriv {
    fn default() -> Self {
        Self {
            battery: TemplateChild::default(),
            label: TemplateChild::default(),
            low_battery_color: RefCell::new("red".to_string()),
            charging_color: RefCell::new("green".to_string()),
            normal_color: RefCell::new("white".to_string()),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for CompactPriv {
    const NAME: &'static str = "PowerCompactWidget";
    type Type = Compact;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        // if you use custom widgets from core you need to ensure the type
        Battery::ensure_type();
        klass.set_layout_manager_type::<BinLayout>();
        klass.bind_template();
        // Warning: template callbacks only work if the module is embedded
        // so don't call `klass.bind_template_instance_callbacks();` or dynisland will crash
        // manually connect signals in `ObjectImpl::constructed` instead
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for CompactPriv {
    fn constructed(&self) {
        self.parent_constructed();
    }

    fn dispose(&self) {
        while let Some(child) = self.obj().first_child() {
            child.unparent();
        }
        self.dispose_template();
    }
}

impl WidgetImpl for CompactPriv {}

impl Compact {
    /// registered properties:
    /// * `percentage`: `f64`
    /// * `charging`: `bool`
    pub fn new(activity: &mut DynamicActivity) -> Self {
        let this: Self = Object::builder().build();

        // register the property if it doesn't exist
        // this way we can update multiple widgets with the same property
        let _ = activity.add_dynamic_property("percentage", 0.0_f64);

        let compact = this.clone();
        activity
            .subscribe_to_property("percentage", move |new_value| {
                let perc = *cast_dyn_any!(new_value, f64).unwrap();
                compact.imp().battery.set_percentage(perc);
                if !compact.imp().battery.charging() {
                    if compact.imp().battery.percentage() < 0.2 {
                        compact
                            .imp()
                            .battery
                            .set_fill_color(compact.imp().low_battery_color.borrow().clone());
                    } else {
                        compact
                            .imp()
                            .battery
                            .set_fill_color(compact.imp().normal_color.borrow().clone());
                    }
                } else {
                    compact
                        .imp()
                        .battery
                        .set_fill_color(compact.imp().charging_color.borrow().clone());
                }
            })
            .unwrap();

        let _ = activity.add_dynamic_property("charging", false);

        let compact = this.clone();
        activity
            .subscribe_to_property("charging", move |new_value| {
                let charging = *cast_dyn_any!(new_value, bool).unwrap();
                compact.imp().battery.set_charging(charging);
                if charging {
                    compact
                        .imp()
                        .battery
                        .set_fill_color(compact.imp().charging_color.borrow().clone());
                } else {
                    if compact.imp().battery.percentage() < 0.2 {
                        compact
                            .imp()
                            .battery
                            .set_fill_color(compact.imp().low_battery_color.borrow().clone());
                    } else {
                        compact
                            .imp()
                            .battery
                            .set_fill_color(compact.imp().normal_color.borrow().clone());
                    }
                }
            })
            .unwrap();

        let _ = activity
            .add_dynamic_property("time-to", (upower::device::State::Unknown, 0_u64, 0_u64));

        let compact = this.clone();
        activity
            .subscribe_to_property("time-to", move |new_value| {
                let (state, time_to_empty, time_to_full) =
                    *cast_dyn_any!(new_value, (upower::device::State, u64, u64)).unwrap();
                let charging = matches!(
                    state,
                    upower::device::State::FullyCharged | upower::device::State::Charging
                );
                if charging {
                    let h = time_to_full / 3600;
                    let m = (time_to_full % 3600) / 60;
                    if matches!(state, upower::device::State::FullyCharged) {
                        compact.imp().label.set_text("Fully Charged");
                        compact.imp().label.set_width_chars(12);
                    } else if time_to_full == 0 {
                        compact.imp().label.set_text("Charging");
                        compact.imp().label.set_width_chars(7);
                    } else {
                        compact.imp().label.set_text(&format!("{h}h{m}m to full"));
                        compact.imp().label.set_width_chars(12);
                    }
                } else {
                    let h = time_to_empty / 3600;
                    let m = (time_to_empty % 3600) / 60;
                    if time_to_empty == 0 {
                        compact.imp().label.set_text("Discharging");
                        compact.imp().label.set_width_chars(10);
                    } else {
                        compact.imp().label.set_text(&format!("{h}h{m}m to empty"));
                        compact.imp().label.set_width_chars(13);
                    }
                }
                //TODO display unknown state
            })
            .unwrap();
        this
    }
}
