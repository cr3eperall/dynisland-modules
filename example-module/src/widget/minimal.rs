glib::wrapper! {
    pub struct Minimal(ObjectSubclass<MinimalPriv>)
    @extends gtk::Widget;
}
use dynisland_core::{
    cast_dyn_any, dynamic_activity::DynamicActivity,
    graphics::widgets::scrolling_label::ScrollingLabel,
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
#[template(resource = "/com/github/cr3eperall/dynislandModules/exampleModule/minimal.ui")]
pub struct MinimalPriv {
    #[template_child]
    pub scroll: TemplateChild<ScrollingLabel>,
}

#[glib::object_subclass]
impl ObjectSubclass for MinimalPriv {
    const NAME: &'static str = "MinimalWidget";
    type Type = Minimal;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        ScrollingLabel::ensure_type();
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for MinimalPriv {
    fn constructed(&self) {
        // Call "constructed" on parent
        self.parent_constructed();
    }
}

impl WidgetImpl for MinimalPriv {
    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        self.obj()
            .first_child()
            .unwrap()
            .size_allocate(&gdk::Rectangle::new(0, 0, width, height), baseline);
    }
}

impl Minimal {
    /// registered properties:
    /// * `scrolling-label-text`: `String`
    pub fn new(activity: &mut DynamicActivity) -> Self {
        let this: Self = Object::builder().build();

        activity
            .add_dynamic_property("scrolling-label-text", "Hello, World".to_string())
            .unwrap();

        let minimal = this.clone();
        activity
            .subscribe_to_property("scrolling-label-text", move |new_value| {
                let real_value = cast_dyn_any!(new_value, String).unwrap();
                log::trace!("text changed:{real_value}");
                minimal.imp().scroll.set_text(real_value.as_str());
            })
            .unwrap();

        this
    }
}
