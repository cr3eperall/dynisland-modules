//! Proxies for DBus services.
//!
//! The interface XML files were taken from
//! [upower](https://gitlab.freedesktop.org/upower/upower/-/tree/master/dbus) and the proxies were
//! generated with [zbus-xmlgen](https://docs.rs/crate/zbus_xmlgen/latest) by running
//! `zbus-xmlgen file dynisland-modules/power-module/src/upower/proxy/org.freedesktop.UPower.xml`,
//! `zbus-xmlgen file dynisland-modules/power-module/src/upower/proxy/org.freedesktop.UPower.Device.xml`
//! `zbus-xmlgen file dynisland-modules/power-module/src/upower/proxy/org.freedesktop.UPower.KbdBacklight.xml`.
//!
//! kbd_backlight.rs is not currently used, but it is included for completeness.
//!
//! For more information, see ["Writing a client proxy" in the zbus
//! tutorial](https://dbus2.github.io/zbus/).

pub mod device;
pub mod kbd_backlight;
pub mod upower;
