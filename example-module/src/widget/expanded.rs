glib::wrapper! {
    pub struct Expanded(ObjectSubclass<ExpandedPriv>)
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
#[template(resource = "/com/github/cr3eperall/dynislandModules/exampleModule/expanded.ui")]
pub struct ExpandedPriv {}

#[glib::object_subclass]
impl ObjectSubclass for ExpandedPriv {
    const NAME: &'static str = "ExpandedWidget";
    type Type = Expanded;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for ExpandedPriv {
    fn constructed(&self) {
        // Call "constructed" on parent
        self.parent_constructed();
    }
}

impl WidgetImpl for ExpandedPriv {
    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        self.obj()
            .first_child()
            .unwrap()
            .size_allocate(&gdk::Rectangle::new(0, 0, width, height), baseline);
    }
}

impl Expanded {
    pub fn new() -> Self {
        Object::builder().build()
    }
}

impl Default for Expanded {
    fn default() -> Self {
        Self::new()
    }
}
