use std::cell::RefCell;

use dynisland_core::{
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
    graphics::{activity_widget::ActivityWidget, widgets::scrolling_label::ScrollingLabel},
};
use glib::{
    object::ObjectExt,
    subclass::{
        object::{DerivedObjectProperties, ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
        InitializingObject,
    },
    types::StaticTypeExt,
    Object, Properties,
};
use gtk::{
    prelude::*,
    subclass::widget::{
        CompositeTemplateClass, CompositeTemplateDisposeExt, CompositeTemplateInitializingExt,
        WidgetClassExt, WidgetImpl,
    },
    BinLayout, CompositeTemplate, TemplateChild,
};

glib::wrapper! {
    pub struct Compact(ObjectSubclass<CompactPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate, Default, Properties)]
#[properties(wrapper_type = Compact)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/scriptModule/compact.ui")]
pub struct CompactPriv {
    #[template_child]
    pub label: TemplateChild<gtk::Label>,
    #[template_child]
    pub scroll: TemplateChild<ScrollingLabel>,
    #[property(get, set)]
    scrolling: RefCell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for CompactPriv {
    const NAME: &'static str = "ScriptCompactWidget";
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
        while let Some(child) = self.obj().first_child() {
            child.unparent();
        }
        self.dispose_template();
    }
}

impl WidgetImpl for CompactPriv {}

impl Compact {
    /// registered properties:
    /// * `compact-text`: `String`
    /// * `scrolling`: `bool`
    /// * `scrolling-speed`: `f32`
    /// * `max-width`: `u32`
    //TODO * `scrolling-delay`: `f32`
    pub fn new(activity: &mut DynamicActivity, aw: &ActivityWidget) -> Self {
        let this: Self = Object::builder().build();
        let aw = aw.clone();
        //text
        if activity.get_property_any("compact-text").is_err() {
            activity
                .add_dynamic_property("compact-text", String::new())
                .unwrap();
        }
        let label = this.imp().label.clone();
        let scroll = this.imp().scroll.clone();
        activity
            .subscribe_to_property("compact-text", move |value| {
                let text = cast_dyn_any!(value, String).unwrap();

                label.set_label(text);
                scroll.set_text(text.clone());
                aw.queue_resize();
            })
            .unwrap();

        //scroll
        if activity.get_property_any("scrolling").is_err() {
            activity.add_dynamic_property("scrolling", false).unwrap();
        }
        let label = this.imp().label.clone();
        let scroll = this.imp().scroll.clone();
        activity
            .subscribe_to_property("scrolling", move |value| {
                let scrolling = *cast_dyn_any!(value, bool).unwrap();

                label.set_visible(!scrolling);
                scroll.set_visible(scrolling);
            })
            .unwrap();

        //max-width
        if activity.get_property_any("max-width").is_err() {
            activity.add_dynamic_property("max-width", 30_i32).unwrap();
        }
        let label = this.imp().label.clone();
        let scroll = this.imp().scroll.clone();
        activity
            .subscribe_to_property("max-width", move |value| {
                let max_width = *cast_dyn_any!(value, i32).unwrap();

                label.set_max_width_chars(max_width);
                scroll.set_max_width(max_width);
            })
            .unwrap();

        //scrolling-speed
        if activity.get_property_any("scrolling-speed").is_err() {
            activity
                .add_dynamic_property("scrolling-speed", 30_f32)
                .unwrap();
        }
        let scroll = this.imp().scroll.clone();
        activity
            .subscribe_to_property("scrolling-speed", move |value| {
                let scroll_speed = *cast_dyn_any!(value, f32).unwrap();

                scroll.set_config_scroll_speed(scroll_speed);
            })
            .unwrap();

        this
    }
}
