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

glib::wrapper! {
    pub struct Minimal(ObjectSubclass<MinimalPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate, Properties)]
#[properties(wrapper_type = Minimal)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/powerModule/minimal.ui")]
pub struct MinimalPriv {
    #[template_child]
    pub battery: TemplateChild<Battery>,
    #[property(get, set)]
    pub low_battery_color: RefCell<String>,
    #[property(get, set)]
    pub charging_color: RefCell<String>,
    #[property(get, set)]
    pub normal_color: RefCell<String>,
    //TODO add low battery treshold
}

impl Default for MinimalPriv {
    fn default() -> Self {
        Self {
            battery: TemplateChild::default(),
            low_battery_color: RefCell::new("red".to_string()),
            charging_color: RefCell::new("green".to_string()),
            normal_color: RefCell::new("white".to_string()),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for MinimalPriv {
    const NAME: &'static str = "PowerMinimalWidget";
    type Type = Minimal;
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
impl ObjectImpl for MinimalPriv {
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

impl WidgetImpl for MinimalPriv {}

impl Minimal {
    /// registered properties:
    /// * `percentage`: `f64`
    /// * `charging`: `bool`
    pub fn new(activity: &mut DynamicActivity) -> Self {
        let this: Self = Object::builder().build();

        // register the property if it doesn't exist
        // this way we can update multiple widgets with the same property
        let _ = activity.add_dynamic_property("percentage", 0.0_f64);

        let minimal = this.clone();
        activity
            .subscribe_to_property("percentage", move |new_value| {
                let perc = *cast_dyn_any!(new_value, f64).unwrap();
                minimal.imp().battery.set_percentage(perc);
                if !minimal.imp().battery.charging() {
                    if minimal.imp().battery.percentage() < 0.2 {
                        minimal
                            .imp()
                            .battery
                            .set_fill_color(minimal.imp().low_battery_color.borrow().clone());
                    } else {
                        minimal
                            .imp()
                            .battery
                            .set_fill_color(minimal.imp().normal_color.borrow().clone());
                    }
                } else {
                    minimal
                        .imp()
                        .battery
                        .set_fill_color(minimal.imp().charging_color.borrow().clone());
                }
            })
            .unwrap();

        let _ = activity.add_dynamic_property("charging", false);

        let minimal = this.clone();
        activity
            .subscribe_to_property("charging", move |new_value| {
                let charging = *cast_dyn_any!(new_value, bool).unwrap();
                minimal.imp().battery.set_charging(charging);
                if charging {
                    minimal
                        .imp()
                        .battery
                        .set_fill_color(minimal.imp().charging_color.borrow().clone());
                } else {
                    if minimal.imp().battery.percentage() < 0.2 {
                        minimal
                            .imp()
                            .battery
                            .set_fill_color(minimal.imp().low_battery_color.borrow().clone());
                    } else {
                        minimal
                            .imp()
                            .battery
                            .set_fill_color(minimal.imp().normal_color.borrow().clone());
                    }
                }
            })
            .unwrap();
        this
    }
}
