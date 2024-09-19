use dynisland_core::{
    abi::{glib, gtk},
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
    graphics::widgets::rolling_char::RollingChar,
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
#[template(resource = "/com/github/cr3eperall/dynislandModules/systrayModule/minimal.ui")]
pub struct MinimalPriv {
    #[template_child]
    pub roll0: TemplateChild<RollingChar>,
    #[template_child]
    pub roll1: TemplateChild<RollingChar>,
}

#[glib::object_subclass]
impl ObjectSubclass for MinimalPriv {
    const NAME: &'static str = "SystrayMinimalWidget";
    type Type = Minimal;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        // if you use custom widgets from core you need to ensure the type
        RollingChar::ensure_type();
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
    /// * `count`: `i32`
    pub fn new(activity: &mut DynamicActivity) -> Self {
        let this: Self = Object::builder().build();

        // register the property if it doesn't exist
        // this way we can update multiple widgets with the same property
        let _ = activity.add_dynamic_property("count", 0i32);

        let minimal = this.clone();
        activity
            .subscribe_to_property("count", move |new_value| {
                let value_int = *cast_dyn_any!(new_value, i32).unwrap();
                let value_string = format!("{:>2}", value_int % 100);
                let show_0 = value_int > 9;
                minimal
                    .imp()
                    .roll0
                    .set_current_char(value_string.chars().nth(0).unwrap());
                minimal.imp().roll0.set_visible(show_0);
                minimal
                    .imp()
                    .roll1
                    .set_current_char(value_string.chars().nth(1).unwrap());
            })
            .unwrap();

        this
    }
}
