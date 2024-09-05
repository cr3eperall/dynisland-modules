use std::cell::RefCell;

use dynisland_core::{
    abi::{gdk, glib, gtk, log},
    graphics::activity_widget::boxed_activity_mode::ActivityMode,
};
use gdk::{gdk_pixbuf::Pixbuf, gio::MemoryInputStream, prelude::ListModelExtManual};
use glib::{
    object::{Cast, ObjectExt},
    subclass::{
        object::{DerivedObjectProperties, ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
        InitializingObject,
    },
    Bytes, Object, Properties,
};
use gtk::{
    prelude::WidgetExt,
    subclass::widget::{
        CompositeTemplateClass, CompositeTemplateDisposeExt, CompositeTemplateInitializingExt,
        WidgetClassExt, WidgetImpl,
    },
    BinLayout, CompositeTemplate, TemplateChild,
};

use crate::utils::{format_rgb_color, remap_num};

glib::wrapper! {
    pub struct Visualizer(ObjectSubclass<VisualizerPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate, Properties, Default)]
#[properties(wrapper_type = Visualizer)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/musicModule/visualizer.ui")]
pub struct VisualizerPriv {
    #[template_child]
    pub container: TemplateChild<gtk::Box>,

    #[property(get, set, default_value = 30)]
    pub width: RefCell<i32>,
    #[property(get, set, default_value = 30)]
    pub height: RefCell<i32>,
}

#[glib::object_subclass]
impl ObjectSubclass for VisualizerPriv {
    const NAME: &'static str = "MusicVisualizerWidget";
    type Type = Visualizer;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.set_layout_manager_type::<BinLayout>();
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for VisualizerPriv {
    fn constructed(&self) {
        self.parent_constructed();
    }
    fn dispose(&self) {
        while let Some(child) = self.obj().first_child() {
            child.unparent();
        }
        self.dispose_template();
    }
    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "width" => {
                let width: i32 = value.get().unwrap();
                let this = self.obj();
                this.set_width_request(width);
                let container = this.imp().container.clone();
                container.set_width_request(width);

                let box_size = (width as f32 / 12.0) as i32;

                for child in container
                    .observe_children()
                    .iter::<glib::Object>()
                    .flatten()
                {
                    let box_n = child.downcast::<gtk::Box>().unwrap();
                    box_n.set_size_request(box_size, box_size);
                }
            }
            "height" => {
                let height: i32 = value.get().unwrap();
                let this = self.obj();
                this.set_height_request(height);
                let container = this.imp().container.clone();
                container.set_height_request(height);
            }
            _ => {
                log::warn!("Visualizer: invalid property received: {}", pspec.name());
            }
        }
    }
}

impl WidgetImpl for VisualizerPriv {}

impl Visualizer {
    pub fn new(width: i32, height: i32) -> Self {
        let this: Self = Object::builder().build();
        this.set_size_request(width, height);
        let container = this.imp().container.clone();
        container.set_size_request(width, height);

        let box_size = (width as f32 / 12.0) as i32;

        for child in container
            .observe_children()
            .iter::<glib::Object>()
            .flatten()
        {
            let box_n = child.downcast::<gtk::Box>().unwrap();
            box_n.set_size_request(box_size, box_size);
        }

        this
    }
}

/// Input format: "1,2,3,4,5,6\n"
pub fn parse_input(line: &str) -> [u8; 6] {
    let line: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    let vec: Vec<u8> = line
        .split(',')
        .map(|num| num.parse::<u8>().unwrap_or(0_u8))
        .collect();
    if vec.len() != 6 {
        return [0_u8; 6];
    }
    [vec[0], vec[1], vec[2], vec[3], vec[4], vec[5]]
}

pub fn get_bar_css(
    css_class: &str,
    cava_data: &[u8; 6],
    pre_max_height: u8,
    post_max_height_compact: u8,
    post_max_height_expanded: u8,
    mode: ActivityMode,
) -> String {
    let post_max_height = match mode {
        ActivityMode::Compact => post_max_height_compact,
        ActivityMode::Expanded => post_max_height_expanded,
        _ => {
            return "".to_string();
        }
    };
    let mode = mode.to_string();
    let (d0, d1, d2, d3, d4, d5) = (
        remap_num(cava_data[0], 0, pre_max_height, 0, post_max_height),
        remap_num(cava_data[1], 0, pre_max_height, 0, post_max_height),
        remap_num(cava_data[2], 0, pre_max_height, 0, post_max_height),
        remap_num(cava_data[3], 0, pre_max_height, 0, post_max_height),
        remap_num(cava_data[4], 0, pre_max_height, 0, post_max_height),
        remap_num(cava_data[5], 0, pre_max_height, 0, post_max_height),
    );
    format!(
        r"
        .{css_class} .mode-{mode} .visualizer .bar-0{{
            min-height: {d0}px;
        }}
        .{css_class} .mode-{mode} .visualizer .bar-1{{
            min-height: {d1}px;
        }}
        .{css_class} .mode-{mode} .visualizer .bar-2{{
            min-height: {d2}px;
        }}
        .{css_class} .mode-{mode} .visualizer .bar-3{{
            min-height: {d3}px;
        }}
        .{css_class} .mode-{mode} .visualizer .bar-4{{
            min-height: {d4}px;
        }}
        .{css_class} .mode-{mode} .visualizer .bar-5{{
            min-height: {d5}px;
        }}
    "
    )
}

pub fn get_gradient_css(css_class: &str, gradient_mat: &[[[u8; 3]; 6]; 3]) -> String {
    let (c00, c01, c02) = (
        format_rgb_color(gradient_mat[0][0]),
        format_rgb_color(gradient_mat[1][0]),
        format_rgb_color(gradient_mat[2][0]),
    );
    let (c10, c11, c12) = (
        format_rgb_color(gradient_mat[0][1]),
        format_rgb_color(gradient_mat[1][1]),
        format_rgb_color(gradient_mat[2][1]),
    );
    let (c20, c21, c22) = (
        format_rgb_color(gradient_mat[0][2]),
        format_rgb_color(gradient_mat[1][2]),
        format_rgb_color(gradient_mat[2][2]),
    );
    let (c30, c31, c32) = (
        format_rgb_color(gradient_mat[0][3]),
        format_rgb_color(gradient_mat[1][3]),
        format_rgb_color(gradient_mat[2][3]),
    );
    let (c40, c41, c42) = (
        format_rgb_color(gradient_mat[0][4]),
        format_rgb_color(gradient_mat[1][4]),
        format_rgb_color(gradient_mat[2][4]),
    );
    let (c50, c51, c52) = (
        format_rgb_color(gradient_mat[0][5]),
        format_rgb_color(gradient_mat[1][5]),
        format_rgb_color(gradient_mat[2][5]),
    );
    format!(
        r"
        .{css_class} .visualizer .bar-0{{
            background-image: linear-gradient(to bottom, {c00}, {c01}, {c02});
        }}
        .{css_class} .visualizer .bar-1{{
            background-image: linear-gradient(to bottom, {c10}, {c11}, {c12});
        }}
        .{css_class} .visualizer .bar-2{{
            background-image: linear-gradient(to bottom, {c20}, {c21}, {c22});
        }}
        .{css_class} .visualizer .bar-3{{
            background-image: linear-gradient(to bottom, {c30}, {c31}, {c32});
        }}
        .{css_class} .visualizer .bar-4{{
            background-image: linear-gradient(to bottom, {c40}, {c41}, {c42});
        }}
        .{css_class} .visualizer .bar-5{{
            background-image: linear-gradient(to bottom, {c50}, {c51}, {c52});
        }}
    "
    )
}

pub fn gradient_from_image_bytes(data: &Vec<u8>) -> [[[u8; 3]; 6]; 3] {
    if data.is_empty() {
        return [[[255_u8; 3]; 6]; 3];
    }
    let data = data.as_slice();
    let data = Bytes::from(data);
    let mut pixbuf = Pixbuf::from_stream(
        &MemoryInputStream::from_bytes(&data),
        None::<&gtk::gio::Cancellable>,
    )
    .ok();
    if pixbuf.is_none() {
        pixbuf = Pixbuf::new(gdk::gdk_pixbuf::Colorspace::Rgb, false, 8, 6, 3);
    }
    let pixbuf = pixbuf.unwrap();
    //TODO use a better color scheme interpolation method
    let scaled_pixbuf = pixbuf
        .scale_simple(6, 3, gdk::gdk_pixbuf::InterpType::Bilinear)
        .unwrap();
    scaled_pixbuf.saturate_and_pixelate(&scaled_pixbuf, 1.5, false);
    unsafe {
        let pixel_bytes = scaled_pixbuf
            .pixels()
            .chunks(scaled_pixbuf.rowstride().try_into().unwrap());
        if pixel_bytes.len() != 3 {
            return [[[255_u8; 3]; 6]; 3];
        }
        let rows: Vec<[[u8; 3]; 6]> = pixel_bytes
            .map(|row| {
                if scaled_pixbuf.has_alpha() {
                    row.chunks_exact(4)
                        .map(|val| [val[0], val[1], val[2]])
                        .collect()
                } else {
                    row.chunks_exact(3)
                        .map(|val| [val[0], val[1], val[2]])
                        .collect()
                }
            })
            .map(|v: Vec<[u8; 3]>| [v[0], v[1], v[2], v[3], v[4], v[5]])
            .collect();

        [rows[0], rows[1], rows[2]]
    }
}
