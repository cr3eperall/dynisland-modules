use dynisland_core::{
    abi::{glib, gtk},
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
};
use glib::{
    subclass::{
        object::{ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
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

use crate::utils::ImageType;

glib::wrapper! {
    pub struct Minimal(ObjectSubclass<MinimalPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate, Default)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/scriptModule/minimal.ui")]
pub struct MinimalPriv {
    #[template_child]
    pub image: TemplateChild<gtk::Image>,
}

#[glib::object_subclass]
impl ObjectSubclass for MinimalPriv {
    const NAME: &'static str = "ScriptMinimalWidget";
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
    /// registered properties:
    /// * `image`: `ImageType`
    pub fn new(activity: &mut DynamicActivity) -> Self {
        let this: Self = Object::builder().build();
        let image = this.imp().image.clone();

        if activity.get_property_any("image").is_err() {
            activity
                .add_dynamic_property(
                    "image",
                    ImageType::Icon(String::from("image-missing-symbolic")),
                )
                .unwrap();
        }
        activity
            .subscribe_to_property("image", move |value| {
                let image_type = cast_dyn_any!(value, ImageType).unwrap();
                match image_type {
                    ImageType::Texture(tex) => {
                        image.set_from_paintable(Some(tex));
                    }
                    ImageType::File(path) => {
                        image.set_from_file(Some(path));
                    }
                    ImageType::Icon(name) => {
                        image.set_from_icon_name(Some(name));
                    }
                }
            })
            .unwrap();

        this
    }
}
