use std::{fmt::Display, thread};

use dynisland_core::abi::{gtk, log};

use super::proxy::dbus_status_notifier_item::StatusNotifierItemProxy;

#[derive(Debug)]
pub enum IconError {
    DBusError(zbus::Error),
    LoadIconFromFile {
        path: String,
        source: gtk::glib::Error,
    },
    #[allow(dead_code)]
    LoadIconFromTheme {
        icon_name: String,
        theme_path: Option<String>,
        source: gtk::glib::Error,
    },
    NotAvailable,
}

impl Display for IconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IconError::DBusError(e) => write!(f, "DBus error: {}", e),
            IconError::LoadIconFromFile { path, source } => {
                write!(f, "Failed to load icon from file {}: {}", path, source)
            }
            IconError::LoadIconFromTheme {
                icon_name,
                theme_path,
                source,
            } => write!(
                f,
                "Failed to load icon {} from theme {:?}: {}",
                icon_name, theme_path, source
            ),
            IconError::NotAvailable => write!(f, "Icon not available"),
        }
    }
}
#[derive(Debug, Clone)]
pub enum IconType {
    Icon,
    AttentionIcon,
    OverlayIcon,
}

impl Display for IconType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IconType::Icon => write!(f, "Icon"),
            IconType::AttentionIcon => write!(f, "AttentionIcon"),
            IconType::OverlayIcon => write!(f, "OverlayIcon"),
        }
    }
}

/// Get the fallback GTK icon, as a final fallback if the tray item has no icon.
async fn fallback_icon(size: i32, scale: i32) -> gtk::gdk::Paintable {
    let theme = gtk::IconTheme::default();
    theme
        .lookup_icon(
            "image-missing",
            vec![].as_slice(),
            size,
            scale,
            gtk::TextDirection::None,
            gtk::IconLookupFlags::empty(),
        )
        .into()
}

/// Load a pixbuf from StatusNotifierItem's [Icon format].
///
/// [Icon format]: https://freedesktop.org/wiki/Specifications/StatusNotifierItem/Icons/
fn icon_from_pixmap(width: i32, height: i32, mut data: Vec<u8>) -> gtk::gdk_pixbuf::Pixbuf {
    // We need to convert data from ARGB32 to RGBA32, since that's the only one that gdk-pixbuf
    // understands.
    for chunk in data.chunks_exact_mut(4) {
        let a = chunk[0];
        let r = chunk[1];
        let g = chunk[2];
        let b = chunk[3];
        chunk[0] = r;
        chunk[1] = g;
        chunk[2] = b;
        chunk[3] = a;
    }

    gtk::gdk_pixbuf::Pixbuf::from_bytes(
        &gtk::glib::Bytes::from_owned(data),
        gtk::gdk_pixbuf::Colorspace::Rgb,
        true,
        8,
        width,
        height,
        width * 4,
    )
}

/// From a list of pixmaps, create an icon from the most appropriately sized one.
///
/// This function returns None if and only if no pixmaps are provided.
fn icon_from_pixmaps(
    pixmaps: Vec<(i32, i32, Vec<u8>)>,
    size: i32,
) -> Option<gtk::gdk_pixbuf::Pixbuf> {
    pixmaps
        .into_iter()
        .max_by(|(w1, h1, _), (w2, h2, _)| {
            // take smallest one bigger than requested size, otherwise take biggest
            let a = size * size;
            let a1 = w1 * h1;
            let a2 = w2 * h2;
            match (a1 >= a, a2 >= a) {
                (true, true) => a2.cmp(&a1),
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                (false, false) => a1.cmp(&a2),
            }
        })
        .and_then(|(w, h, d)| {
            let pixbuf = icon_from_pixmap(w, h, d);
            if w != size || h != size {
                pixbuf.scale_simple(size, size, gtk::gdk_pixbuf::InterpType::Bilinear)
            } else {
                Some(pixbuf)
            }
        })
}

/// Load an icon with a given name from either the default (if `theme_path` is `None`), or from the
/// theme at a path.
pub fn icon_from_name(
    icon_name: &str,
    theme_path: Option<&str>,
    size: i32,
    scale: i32,
) -> std::result::Result<gtk::gdk::Paintable, IconError> {
    let theme = if let Some(path) = theme_path {
        let theme = gtk::IconTheme::new();
        theme.add_search_path(path);
        theme
    } else {
        let theme =gtk::IconTheme::default();
        let mut env_theme_name=std::env::var("GTK_THEME").unwrap_or_else(|_| "Adwaita".to_string());
        // remove the variant part of the theme name
        env_theme_name= env_theme_name.split_once(":").map(|(pre,_post)|pre.to_string()).unwrap_or(env_theme_name);
        theme.set_theme_name(Some(&env_theme_name));
        theme
    };

    Ok(theme
        .lookup_icon(
            icon_name,
            vec![].as_slice(),
            size,
            scale,
            gtk::TextDirection::None,
            gtk::IconLookupFlags::empty(),
        )
        .into())
}

pub async fn load_icon_from_sni(
    sni: &StatusNotifierItemProxy<'static>,
    size: i32,
    scale: i32,
    icon: IconType,
) -> Option<gtk::gdk::Paintable> {
    // "Visualizations are encouraged to prefer icon names over icon pixmaps if both are
    // available."

    let scaled_size = size * scale;

    // First, see if we can get an icon from the name they provide, using either the theme they
    // specify or the default.
    let icon_from_name: std::result::Result<gtk::gdk::Paintable, IconError> = (async {
        // fetch icon name
        let icon_name = match icon {
            IconType::Icon => sni.icon_name().await,
            IconType::AttentionIcon => sni.attention_icon_name().await,
            IconType::OverlayIcon => sni.overlay_icon_name().await,
        };
        let icon_name = match icon_name {
            Ok(s) if s.is_empty() => return Err(IconError::NotAvailable),
            Ok(s) => s,
            Err(e) => return Err(IconError::DBusError(e)),
        };

        // interpret it as an absolute path if we can
        let icon_path = std::path::Path::new(&icon_name);
        if icon_path.is_absolute() && icon_path.is_file() {
            return gtk::gdk_pixbuf::Pixbuf::from_file_at_size(icon_path, scaled_size, scaled_size)
                .map_err(|e| IconError::LoadIconFromFile {
                    path: icon_name,
                    source: e,
                })
                .map(|pb| gtk::gdk::Texture::for_pixbuf(&pb).into());
        }

        // otherwise, fetch icon theme and lookup using icon_from_name
        let icon_theme_path = sni.icon_theme_path().await;
        let icon_theme_path = match icon_theme_path {
            Ok(p) if p.is_empty() => None,
            Ok(p) => Some(p),
            // treat property not existing as the same as it being empty i.e. to use the default
            // system theme
            Err(zbus::Error::FDO(e)) => match *e {
                zbus::fdo::Error::UnknownProperty(_) | zbus::fdo::Error::InvalidArgs(_) => None,
                // this error is reported by discord, blueman-applet
                zbus::fdo::Error::Failed(msg) if msg == "error occurred in Get" => None,
                _ => return Err(IconError::DBusError(zbus::Error::FDO(e))),
            },
            Err(e) => return Err(IconError::DBusError(e)),
        };

        let icon_theme_path: Option<&str> = match &icon_theme_path {
            // this looks weird but this converts &String to &str
            Some(s) => Some(s),
            None => None,
        };
        icon_from_name(&icon_name, icon_theme_path, size, scale)
    })
    .await;

    match icon_from_name {
        Ok(p) => return Some(p),           // got an icon!
        Err(IconError::NotAvailable) => {} // this error is expected, don't log
        Err(e) => log::debug!(
            "failed to get icon by name for {}: {}, {}",
            sni.inner().destination(),
            e,
            icon
        ),
    };

    log::trace!("cant get it from name, trying pixmap");

    // Can't get it from name + theme, try the pixmap
    let (pixmap_tx, pixmap_rx) =
        tokio::sync::oneshot::channel::<zbus::Result<Vec<(i32, i32, Vec<u8>)>>>();
    thread::spawn({
        let icon = icon.clone();
        let sni = sni.clone();
        move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on({
                let sni = sni.clone();
                let icon = icon.clone();
                async move {
                    // These are async functions, but they are transfering a lot of data and block the thread, so we run them in a separate thread.
                    let pixmap = match icon {
                        IconType::Icon => sni.icon_pixmap().await,
                        IconType::AttentionIcon => sni.attention_icon_pixmap().await,
                        IconType::OverlayIcon => sni.overlay_icon_pixmap().await,
                    };
                    pixmap_tx.send(pixmap).unwrap();
                }
            })
        }
    });
    let pixmap = match pixmap_rx.await {
        Ok(res) => res,
        Err(err) => Err(zbus::Error::Failure(format!(
            "Failed to get icon pixmap: {:?}",
            err
        ))),
    };

    if pixmap.is_ok() {
        log::trace!(
            "dbus: {} pixmap -> Ok({} pixmaps)",
            sni.inner().destination(),
            pixmap.as_ref().unwrap().len()
        );
    }

    let icon_from_pixmaps = match pixmap {
        Ok(ps) => match icon_from_pixmaps(ps, scaled_size) {
            Some(p) => Ok(p),
            None => Err(IconError::NotAvailable),
        },
        Err(zbus::Error::FDO(e)) => match *e {
            // property not existing is an expected error
            zbus::fdo::Error::UnknownProperty(_) | zbus::fdo::Error::InvalidArgs(_) => {
                Err(IconError::NotAvailable)
            }

            _ => Err(IconError::DBusError(zbus::Error::FDO(e))),
        },
        Err(e) => Err(IconError::DBusError(e)),
    };
    match icon_from_pixmaps {
        Ok(p) => return Some(gtk::gdk::Texture::for_pixbuf(&p).into()),
        Err(IconError::NotAvailable) => {}
        Err(e) => log::debug!(
            "failed to get icon pixmap for {}: {}",
            sni.inner().destination(),
            e
        ),
    };

    // Tray didn't provide a valid icon so use the default fallback one.
    match icon {
        IconType::Icon => return Some(fallback_icon(size, scale).await),
        IconType::AttentionIcon | IconType::OverlayIcon => return None,
    }
}
