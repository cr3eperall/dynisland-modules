use dynisland_core::abi::{glib, gtk};
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
    subclass::widget::{
        CompositeTemplateClass, CompositeTemplateDisposeExt, CompositeTemplateInitializingExt,
        WidgetClassExt, WidgetImpl,
    },
    BinLayout, CompositeTemplate,
};

#[derive(CompositeTemplate, Default)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/exampleModule/expanded.ui")]
pub struct ExpandedPriv {}

#[glib::object_subclass]
impl ObjectSubclass for ExpandedPriv {
    const NAME: &'static str = "ExampleExpandedWidget";
    type Type = Expanded;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.set_layout_manager_type::<BinLayout>();
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for ExpandedPriv {
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

impl WidgetImpl for ExpandedPriv {}

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
