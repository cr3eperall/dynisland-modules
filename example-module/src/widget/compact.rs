glib::wrapper! {
    pub struct Compact(ObjectSubclass<CompactPriv>)
    @extends gtk::Widget;
}
use dynisland_core::{
    cast_dyn_any, dynamic_activity::DynamicActivity, graphics::widgets::rolling_char::RollingChar,
};
use glib::{
    subclass::{
        object::{ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
        InitializingObject,
    },
    types::StaticTypeExt,
    Object,
};
use gtk::{
    prelude::WidgetExt,
    subclass::widget::{
        CompositeTemplateClass, CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
    },
    CompositeTemplate, TemplateChild,
};

#[derive(CompositeTemplate, Default)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/exampleModule/compact.ui")]
pub struct CompactPriv {
    #[template_child]
    pub label: TemplateChild<gtk::Label>,
    #[template_child]
    pub roll1: TemplateChild<RollingChar>,
    #[template_child]
    pub roll2: TemplateChild<RollingChar>,
}

#[glib::object_subclass]
impl ObjectSubclass for CompactPriv {
    const NAME: &'static str = "CompactWidget";
    type Type = Compact;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        RollingChar::ensure_type();
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for CompactPriv {
    fn constructed(&self) {
        // Call "constructed" on parent
        self.parent_constructed();
    }
}

impl WidgetImpl for CompactPriv {
    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        self.obj()
            .first_child()
            .unwrap()
            .size_allocate(&gdk::Rectangle::new(0, 0, width, height), baseline);
    }
}

impl Compact {
    /// registered properties:
    /// * `comp-label`: `String`
    /// * `rolling-char`: `char`
    pub fn new(activity: &mut DynamicActivity) -> Self {
        let this: Self = Object::builder().build();
        activity
            .add_dynamic_property("comp-label", "compact".to_string())
            .unwrap();

        let compact = this.clone();
        activity
            .subscribe_to_property("comp-label", move |new_value| {
                let real_value = cast_dyn_any!(new_value, String).unwrap();
                compact.imp().label.set_label(real_value);
            })
            .unwrap();

        activity.add_dynamic_property("rolling-char", '0').unwrap();

        let compact = this.clone();
        activity
            .subscribe_to_property("rolling-char", move |new_value| {
                let real_value = cast_dyn_any!(new_value, char).unwrap();

                let rolling_char_1 = &compact.imp().roll1;
                rolling_char_1.set_current_char(real_value);

                let rolling_char_2 = &compact.imp().roll2;
                rolling_char_2.set_current_char(real_value);
            })
            .unwrap();
        this
    }
}
