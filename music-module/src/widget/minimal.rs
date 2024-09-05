use dynisland_core::abi::{glib, gtk};
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
    BinLayout, CompositeTemplate, TemplateChild,
};

glib::wrapper! {
    pub struct Minimal(ObjectSubclass<MinimalPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate, Default)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/musicModule/minimal.ui")]
pub struct MinimalPriv {
    #[template_child]
    pub image: TemplateChild<gtk::Image>,
}

#[glib::object_subclass]
impl ObjectSubclass for MinimalPriv {
    const NAME: &'static str = "MusicMinimalWidget";
    type Type = Minimal;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.set_layout_manager_type::<BinLayout>();
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

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
    pub fn new() -> Self {
        Object::builder().build()
    }
}
