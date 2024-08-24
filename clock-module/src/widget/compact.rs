use std::cell::RefCell;

use chrono::{Local, Timelike};
use dynisland_core::{
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
    graphics::widgets::{rolling_char::RollingChar, scrolling_label::ScrollingLabel},
};
use glib::{
    object::ObjectExt, subclass::object::DerivedObjectProperties, subclass::*,
    types::StaticTypeExt, Object, Properties,
};
use gtk::{subclass::widget::*, BinLayout, CompositeTemplate, TemplateChild};
use object::{ObjectImpl, ObjectImplExt};
use types::{ObjectSubclass, ObjectSubclassIsExt};

glib::wrapper! {
    pub struct Compact(ObjectSubclass<CompactPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate, Properties)]
#[properties(wrapper_type = Compact)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/clockModule/compact.ui")]
pub struct CompactPriv {
    #[template_child]
    pub hour_dec: TemplateChild<RollingChar>,
    #[template_child]
    pub hour_unit: TemplateChild<RollingChar>,
    #[template_child]
    pub minute_dec: TemplateChild<RollingChar>,
    #[template_child]
    pub minute_unit: TemplateChild<RollingChar>,
    #[property(get, set, default_value = true)]
    format_24h: RefCell<bool>,
}

impl Default for CompactPriv {
    fn default() -> Self {
        Self {
            hour_dec: Default::default(),
            hour_unit: Default::default(),
            minute_dec: Default::default(),
            minute_unit: Default::default(),
            format_24h: RefCell::new(true),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for CompactPriv {
    const NAME: &'static str = "ClockCompactWidget";
    type Type = Compact;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        ScrollingLabel::ensure_type();
        klass.set_layout_manager_type::<BinLayout>();
        klass.bind_template();
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
        self.dispose_template();
    }
}

impl WidgetImpl for CompactPriv {}

impl Compact {
    pub fn new(dynamic_activity: &mut DynamicActivity) -> Self {
        let this: Self = Object::builder().build();
        let digital_clock = this.clone();
        if dynamic_activity.get_property_any("time").is_err() {
            dynamic_activity
                .add_dynamic_property("time", Local::now())
                .unwrap();
        }
        dynamic_activity
            .subscribe_to_property("time", move |value| {
                let time = cast_dyn_any!(value, chrono::DateTime<chrono::Local>).unwrap();
                digital_clock.set_time(*time);
            })
            .unwrap();

        this.set_time(Local::now());

        this
    }

    /// must be used from the gtk main context
    pub fn set_time(&self, time: chrono::DateTime<Local>) {
        let imp = self.imp();

        let hour = time.hour() % (if self.format_24h() { 24 } else { 12 });
        let hour = format!("{:>2}", hour);
        let (hour_dec, hour_unit) = (hour.chars().nth(0).unwrap(), hour.chars().nth(1).unwrap());

        let minute = time.minute();
        let minute = format!("{:0>2}", minute);
        let (minute_dec, minute_unit) = (
            minute.chars().nth(0).unwrap(),
            minute.chars().nth(1).unwrap(),
        );

        imp.hour_dec.set_current_char(hour_dec);
        imp.hour_unit.set_current_char(hour_unit);
        imp.minute_dec.set_current_char(minute_dec);
        imp.minute_unit.set_current_char(minute_unit);
    }
}
