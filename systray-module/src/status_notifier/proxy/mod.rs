//! Proxies for DBus services, so we can call them.
//!
//! The interface XML files were taken from
//! [eww](https://github.com/elkowar/eww/tree/master/crates/notifier_host/src/proxy), which in turn were taken from
//! [Waybar](https://github.com/Alexays/Waybar/tree/master/protocol), and the proxies were
//! generated with [zbus-xmlgen](https://docs.rs/crate/zbus_xmlgen/latest) by running
//! `zbus-xmlgen file dynisland-modules/systray-module/src/proxy/dbus_status_notifier_item.xml`,
//! `zbus-xmlgen file dynisland-modules/systray-module/src/proxy/dbus_status_notifier_watcher.xml`
//! `zbus-xmlgen file dynisland-modules/systray-module/src/proxy/dbus_menu.xml`.
//!
//! Note that the `dbus_status_notifier_watcher.rs` file has been slightly adjusted, the
//! default arguments to the [proxy](https://docs.rs/zbus/4.4.0/zbus/attr.proxy.html)
//! macro need some adjusting.
//!
//! For more information, see ["Writing a client proxy" in the zbus
//! tutorial](https://dbus2.github.io/zbus/).

pub mod dbus_menu;
pub mod dbus_status_notifier_item;
pub mod dbus_status_notifier_watcher;
