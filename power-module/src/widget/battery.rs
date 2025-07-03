use std::{cell::RefCell, f64::consts::PI};

use dyn_fmt::AsStrFormatExt;
use dynisland_core::abi::{gdk, glib, gtk, log};
use gdk::RGBA;
use glib::{
    subclass::{
        object::{DerivedObjectProperties, ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt},
    },
    Object, Properties,
};
use gtk::{
    graphene::Rect,
    prelude::*,
    subclass::widget::{WidgetClassExt, WidgetImpl},
    BinLayout,
};

glib::wrapper! {
    pub struct Battery(ObjectSubclass<BatteryPriv>)
    @extends gtk::Widget;
}
#[derive(Properties)]
#[properties(wrapper_type = Battery)]
pub struct BatteryPriv {
    #[property(get, set)]
    percentage: RefCell<f64>,
    #[property(get, set, type=String)]
    fill_color: RefCell<RGBA>,
    #[property(get, set, type=String)]
    background_color: RefCell<RGBA>,
    #[property(get, set)]
    charging: RefCell<bool>,
    #[property(get, set)]
    show_percentage: RefCell<bool>,
    #[property(get, set)]
    #[doc = "Pango markup for the percentage text, the percentage will be inserted in the first `{}` placeholder"]
    percentage_markup: RefCell<String>,
}

#[glib::object_subclass]
impl ObjectSubclass for BatteryPriv {
    const NAME: &'static str = "BatteryWidget";
    type Type = Battery;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.set_layout_manager_type::<BinLayout>();
        klass.set_css_name("battery-widget");
    }
}
#[allow(clippy::derivable_impls)]
impl Default for BatteryPriv {
    fn default() -> Self {
        Self {
            percentage: RefCell::new(0.0),
            fill_color: RefCell::new(RGBA::WHITE),
            background_color: RefCell::new(
                RGBA::builder()
                    .red(0.9)
                    .green(0.9)
                    .blue(0.9)
                    .alpha(0.6)
                    .build(),
            ),
            show_percentage: RefCell::new(true),
            percentage_markup: RefCell::new(
                "<span font_desc=\"Monospace Bold 11\" foreground=\"#FFFFFF00\">{}</span>"
                    .to_string(),
            ),
            charging: RefCell::new(false),
        }
    }
}

#[glib::derived_properties]
impl ObjectImpl for BatteryPriv {
    fn constructed(&self) {
        self.parent_constructed();
        // let battery = self.obj().clone();
        // glib::timeout_add_local(Duration::from_millis(100), move || {
        //     battery.queue_draw();
        //     glib::ControlFlow::Continue
        // });
    }

    fn dispose(&self) {
        while let Some(child) = self.obj().first_child() {
            child.unparent();
        }
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "fill-color" => {
                let name: String = value.get().unwrap();
                if let Ok(color) = RGBA::parse(&name) {
                    self.fill_color.replace(color);
                    self.obj().queue_draw();
                } else {
                    log::warn!("invalid fill color: {name}");
                }
            }
            "background-color" => {
                let name: String = value.get().unwrap();
                if let Ok(color) = RGBA::parse(&name) {
                    self.background_color.replace(color);
                    self.obj().queue_draw();
                } else {
                    log::warn!("invalid background color: {name}");
                }
            }
            "percentage" => {
                let percentage: f64 = value.get().unwrap();
                self.percentage.replace(percentage);
                self.obj().queue_draw();
            }
            "percentage-markup" => {
                let pango_markup: String = value.get().unwrap();
                self.percentage_markup.replace(pango_markup);
                self.obj().queue_draw();
            }
            "charging" => {
                let charging: bool = value.get().unwrap();
                self.charging.replace(charging);
                self.obj().queue_draw();
            }
            "show-percentage" => {
                let show: bool = value.get().unwrap();
                self.show_percentage.replace(show);
                self.obj().queue_draw();
            }
            _ => {
                log::warn!("Battery: invalid property received: {}", pspec.name());
            }
        }
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "fill-color" => self.fill_color.borrow().to_string().to_value(),
            "background-color" => self.background_color.borrow().to_string().to_value(),
            "percentage" => self.percentage.borrow().to_value(),
            "charging" => self.charging.borrow().to_value(),
            "show-percentage" => self.show_percentage.borrow().to_value(),
            _ => self.derived_property(id, pspec),
        }
    }
}

impl WidgetImpl for BatteryPriv {
    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let (w, h) = (self.obj().width() as f64, self.obj().height() as f64);
        let w_h_ratio = 2.0;
        let mut req_h = h;
        let mut req_w = req_h * w_h_ratio;
        let radius = 4.0;
        if req_w > w {
            req_w = w;
            req_h = req_w / w_h_ratio;
        }

        let (x, y) = ((w - req_w) / 2.0, (h - req_h) / 2.0);
        let rect = Rect::new(x as f32, y as f32, req_w as f32, req_h as f32);

        const DEG_0: f64 = 0.0;
        const DEG_90: f64 = PI / 2.0;
        const DEG_180: f64 = PI;
        const DEG_270: f64 = PI * 3.0 / 2.0;
        let ctx = snapshot.append_cairo(&rect);
        let bg_color = self.background_color.borrow();
        ctx.translate(x + 1.0, y + 1.0);

        let req_w = req_w - 2.0;
        let req_h = req_h - 2.0;

        let main_w = req_w * 0.9;
        let terminal_w = req_w * 0.1;
        let percentage = *self.percentage.borrow() as f64;

        ctx.set_fill_rule(gtk::cairo::FillRule::Winding);

        // text
        let perc_markup_template = self.percentage_markup.borrow();
        self.draw_text(&ctx, &perc_markup_template, main_w, req_w, req_h);

        ctx.clip();
        {
            ctx.new_path();

            // main shape (rounded rect)
            ctx.set_line_width(1.0);
            ctx.move_to(radius, 0.0);
            ctx.line_to(main_w - radius, 0.0);
            ctx.arc(main_w - radius, radius, radius, DEG_270, DEG_0);
            ctx.line_to(main_w, req_h - radius);
            ctx.arc(main_w - radius, req_h - radius, radius, DEG_0, DEG_90);
            ctx.line_to(radius, req_h);
            ctx.arc(radius, req_h - radius, radius, DEG_90, DEG_180);
            ctx.line_to(0.0, radius);
            ctx.arc(radius, radius, radius, DEG_180, DEG_270);
            ctx.close_path();

            let main_path = ctx.copy_path().unwrap();

            // battery terminal
            ctx.new_sub_path();
            ctx.move_to(main_w, req_h / 2.0 - terminal_w / 2.0);
            ctx.arc(main_w, req_h / 2.0, terminal_w, -DEG_90, DEG_90);
            ctx.close_path();

            ctx.set_source_color(&bg_color);
            ctx.fill_preserve().unwrap();
            let path_with_terminal = ctx.copy_path().unwrap();

            ctx.clip();
            {
                let fill_color = self.fill_color.borrow();

                // fill
                ctx.rectangle(0.0, 0.0, main_w * percentage, req_h);
                ctx.set_source_color(&fill_color);
                ctx.fill().unwrap();
            }
            ctx.reset_clip();

            // border clip
            ctx.new_path();
            ctx.append_path(&main_path);

            ctx.new_sub_path();
            ctx.move_to(req_w + 1.0, -1.0);
            ctx.line_to(-1.0, -1.0);
            ctx.line_to(-1.0, req_h + 1.0);
            ctx.line_to(req_w + 1.0, req_h + 1.0);
            ctx.close_path();
            ctx.set_fill_rule(gtk::cairo::FillRule::Winding);
            ctx.clip();
            {
                // border line
                ctx.new_path();
                ctx.append_path(&path_with_terminal);
                ctx.set_line_width(1.0);
                ctx.set_source_color(
                    &bg_color
                        .with_red(bg_color.red() * 0.3)
                        .with_green(bg_color.green() * 0.3)
                        .with_blue(bg_color.blue() * 0.3),
                );
                ctx.stroke().unwrap();
            }
            ctx.reset_clip();
        }
        ctx.reset_clip();

        drop(ctx);
    }
}

impl BatteryPriv {
    fn draw_text(
        &self,
        ctx: &gtk::cairo::Context,
        perc_markup_template: &str,
        main_w: f64,
        req_w: f64,
        req_h: f64,
    ) {
        let percentage = *self.percentage.borrow() as f64;
        let perc_markup = perc_markup_template.format(&[format!("{:.0}", percentage * 100.0)]);
        let perc_layout = pangocairo::functions::create_layout(ctx);
        perc_layout.set_markup(&perc_markup);
        let ttr = perc_layout.attributes().unwrap();
        let mut text_color = RGBA::WHITE.with_alpha(1.0);
        for attr in ttr.iterator().attrs().iter() {
            // get all the attributes of the first char to use for all the text
            if attr.type_() == gtk::pango::AttrType::Foreground {
                let color = attr
                    .clone()
                    .downcast::<gtk::pango::AttrColor>()
                    .unwrap()
                    .color();
                text_color.set_red(color.red() as f32 / 65535.0);
                text_color.set_green(color.green() as f32 / 65535.0);
                text_color.set_blue(color.blue() as f32 / 65535.0);
            }
            if attr.type_() == gtk::pango::AttrType::ForegroundAlpha {
                let alpha = attr.clone().downcast::<gtk::pango::AttrInt>().unwrap();
                text_color.set_alpha(alpha.value() as f32 / 65535.0);
            }
        }
        // log::debug!("text_color: {}", text_color.to_string());

        let (perc_w, perc_h) = perc_layout.size();
        let (mut perc_w, mut perc_h) = (
            (perc_w / gtk::pango::SCALE) as f64,
            (perc_h / gtk::pango::SCALE) as f64,
        );
        let (mut charge_w, mut charge_h) = (
            perc_w / format!("{:.0}", percentage * 100.0).len() as f64,
            perc_h,
        );

        if !*self.charging.borrow() {
            charge_w = 0.0;
            charge_h = 0.0;
        }
        if !*self.show_percentage.borrow() {
            perc_w = 0.0;
            perc_h = 0.0;
        }

        // charge symbol
        ctx.new_path();
        if *self.charging.borrow() {
            ctx.save().unwrap();
            ctx.translate(
                (main_w - (perc_w + charge_w)) / 2.0 + perc_w,
                (req_h - charge_h) / 2.0,
            );
            ctx.translate(charge_w * 0.2 / 2.0, charge_h * 0.2 / 2.0);
            ctx.scale(0.8, 0.8);
            ctx.move_to(charge_w * 0.7, 0.0);
            ctx.line_to(0.0, charge_h * 0.6);
            ctx.line_to(charge_w * 0.4, charge_h * 0.6);
            ctx.line_to(charge_w * 0.3, charge_h);
            ctx.line_to(charge_w, charge_h * 0.4);
            ctx.line_to(charge_w * 0.6, charge_h * 0.4);
            ctx.close_path();
            ctx.restore().unwrap();
        }

        // percentage
        if *self.show_percentage.borrow() {
            ctx.new_sub_path();
            // ctx.set_source_color(&RGBA::RED);
            ctx.move_to((main_w - (perc_w + charge_w)) / 2.0, (req_h - perc_h) / 2.0);
            pangocairo::functions::layout_path(ctx, &perc_layout);
        }

        ctx.set_source_color(&text_color);
        ctx.fill_preserve().unwrap();
        // rect
        ctx.new_sub_path();
        ctx.move_to(req_w, 0.0);
        ctx.line_to(req_w, req_h);
        ctx.line_to(0.0, req_h);
        ctx.line_to(0.0, 0.0);
        ctx.close_path();
        // ctx.stroke_preserve().unwrap();
    }
}

#[allow(clippy::new_without_default)]
impl Battery {
    pub fn new() -> Self {
        let this: Self = Object::builder().build();
        this
    }
}
