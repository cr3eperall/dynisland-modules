use dynisland_core::abi::gtk;

use super::{
    icon::load_icon_from_sni, names, proxy::dbus_status_notifier_item::StatusNotifierItemProxy,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusNotifierItemParseError;

/// Recognised values of [`org.freedesktop.StatusNotifierItem.Status`].
///
/// [`org.freedesktop.StatusNotifierItem.Status`]: https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierItem/#org.freedesktop.statusnotifieritem.status
#[derive(Debug, Clone, Copy)]
pub enum Status {
    /// The item doesn't convey important information to the user, it can be considered an "idle"
    /// status and is likely that visualizations will chose to hide it.
    Passive,
    /// The item is active, is more important that the item will be shown in some way to the user.
    Active,
    /// The item carries really important information for the user, such as battery charge running
    /// out and is wants to incentive the direct user intervention. Visualizations should emphasize
    /// in some way the items with NeedsAttention status.
    NeedsAttention,
}

impl std::str::FromStr for Status {
    type Err = StatusNotifierItemParseError;

    fn from_str(s: &str) -> std::result::Result<Self, StatusNotifierItemParseError> {
        match s {
            "Passive" => Ok(Status::Passive),
            "Active" => Ok(Status::Active),
            "NeedsAttention" => Ok(Status::NeedsAttention),
            _ => Err(StatusNotifierItemParseError),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Category {
    /// The item describes the status of a generic application,
    /// for instance the current state of a media player.
    /// In the case where the category of the item can not be known,
    /// such as when the item is being proxied from another incompatible or emulated system,
    /// ApplicationStatus can be used a sensible default fallback.
    ApplicationStatus,
    /// The item describes the status of communication oriented applications,
    /// like an instant messenger or an email client.
    Communications,
    /// The item describes services of the system not seen as a stand alone application by the user,
    /// such as an indicator for the activity of a disk indexing service.
    SystemServices,
    /// The item describes the state and control of a particular hardware,
    /// such as an indicator of the battery charge or sound card volume control.
    Hardware,
}

impl std::str::FromStr for Category {
    type Err = StatusNotifierItemParseError;

    fn from_str(s: &str) -> std::result::Result<Self, StatusNotifierItemParseError> {
        match s {
            "ApplicationStatus" => Ok(Category::ApplicationStatus),
            "Communications" => Ok(Category::Communications),
            "SystemServices" => Ok(Category::SystemServices),
            "Hardware" => Ok(Category::Hardware),
            _ => Err(StatusNotifierItemParseError),
        }
    }
}

/// A StatusNotifierItem (SNI).
///
/// you should directly access the `sni` member as needed for functionalty that is not provided.
#[derive(Debug, Clone)]
pub struct Item {
    /// The StatusNotifierItem that is wrapped by this instance.
    pub sni: StatusNotifierItemProxy<'static>,
}

impl Item {
    /// Create an instance from the service's address.
    ///
    /// The format of `addr` is `{bus}{object_path}` (e.g.
    /// `:1.50/org/ayatana/NotificationItem/nm_applet`), which is the format that is used for
    /// StatusNotifierWatcher's [RegisteredStatusNotifierItems property][rsni]).
    ///
    /// [rsni]: https://freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierWatcher/#registeredstatusnotifieritems
    pub async fn from_address(con: &zbus::Connection, service: &str) -> zbus::Result<Self> {
        let (addr, path) = {
            // Based on <https://github.com/oknozor/stray/blob/main/stray/src/notifier_watcher/notifier_address.rs>
            //
            // TODO is the service name format actually documented anywhere?
            if let Some((addr, path)) = service.split_once('/') {
                (addr.to_owned(), format!("/{}", path))
            } else if service.starts_with(':') {
                (service[0..6].to_owned(), names::ITEM_OBJECT.to_owned())
            } else {
                return Err(zbus::Error::Address(service.to_owned()));
            }
        };

        let sni = StatusNotifierItemProxy::builder(con)
            .destination(addr.clone())?
            .path(path)?
            .build()
            .await?;

        Ok(Self { sni })
    }
}

#[allow(dead_code)]
impl Item {
    // Properties

    /// Get the current status of the item.
    pub async fn status(&self) -> zbus::Result<Status> {
        let status = self.sni.status().await?;
        match status.parse() {
            Ok(s) => Ok(s),
            Err(_) => Err(zbus::Error::Failure(format!("Invalid status {:?}", status))),
        }
    }

    /// Get the current category of the item.
    pub async fn category(&self) -> zbus::Result<Category> {
        let category = self.sni.category().await?;
        match category.parse() {
            Ok(c) => Ok(c),
            Err(_) => Err(zbus::Error::Failure(format!(
                "Invalid category {:?}",
                category
            ))),
        }
    }

    pub async fn id(&self) -> zbus::Result<String> {
        self.sni.id().await
    }

    pub async fn title(&self) -> zbus::Result<String> {
        self.sni.title().await
    }

    pub async fn window_id(&self) -> zbus::Result<u32> {
        self.sni.window_id().await
    }

    pub async fn icon(&self, size: i32, scale: i32) -> gtk::gdk::Paintable {
        load_icon_from_sni(&self.sni, size, scale, super::icon::IconType::Icon)
            .await
            .unwrap()
    }
    pub async fn attention_icon(&self, size: i32, scale: i32) -> Option<gtk::gdk::Paintable> {
        load_icon_from_sni(&self.sni, size, scale, super::icon::IconType::AttentionIcon).await
    }
    pub async fn overlay_icon(&self, size: i32, scale: i32) -> Option<gtk::gdk::Paintable> {
        load_icon_from_sni(&self.sni, size, scale, super::icon::IconType::OverlayIcon).await
    }

    pub async fn tooltip(
        &self,
    ) -> zbus::Result<(String, Vec<(i32, i32, Vec<u8>)>, String, String)> {
        self.sni.tool_tip().await
    }

    pub async fn item_is_menu(&self) -> zbus::Result<bool> {
        self.sni.item_is_menu().await
    }

    pub async fn menu(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath> {
        self.sni.menu().await
    }

    // Methods

    pub async fn scroll(&self, delta: i32, direction_horizontal: bool) -> zbus::Result<()> {
        self.sni
            .scroll(
                delta,
                if direction_horizontal {
                    "horizontal"
                } else {
                    "vertical"
                },
            )
            .await
    }

    pub async fn activate(&self, x: i32, y: i32) -> zbus::Result<()> {
        self.sni.activate(x, y).await
    }

    pub async fn secondary_activate(&self, x: i32, y: i32) -> zbus::Result<()> {
        self.sni.secondary_activate(x, y).await
    }

    pub async fn context_menu(&self, x: i32, y: i32) -> zbus::Result<()> {
        self.sni.context_menu(x, y).await
    }
}
