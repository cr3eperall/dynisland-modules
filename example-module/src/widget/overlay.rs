glib::wrapper! {
    pub struct Overlay(ObjectSubclass<OverlayPriv>)
    @extends gtk::Widget;
}
use glib::{
    subclass::{
        object::{ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt},
        InitializingObject,
    },
    Object,
};
use gtk::{
    prelude::WidgetExt,
    subclass::widget::{CompositeTemplateClass, CompositeTemplateInitializingExt, WidgetImpl},
    CompositeTemplate,
};

#[derive(CompositeTemplate, Default)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/exampleModule/overlay.ui")]
pub struct OverlayPriv {}

#[glib::object_subclass]
impl ObjectSubclass for OverlayPriv {
    const NAME: &'static str = "OverlayWidget";
    type Type = Overlay;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for OverlayPriv {
    fn constructed(&self) {
        // Call "constructed" on parent
        self.parent_constructed();
    }
}

impl WidgetImpl for OverlayPriv {
    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        self.obj()
            .first_child()
            .unwrap()
            .size_allocate(&gdk::Rectangle::new(0, 0, width, height), baseline);
    }
}

impl Overlay {
    pub fn new() -> Self {
        Object::builder().build()
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self::new()
    }
}
