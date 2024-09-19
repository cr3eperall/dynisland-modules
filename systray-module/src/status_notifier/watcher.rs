use std::{collections::HashSet, iter::Iterator, sync::Arc};

use dynisland_core::abi::log;
use tokio::sync::Mutex;
use zbus::{export::ordered_stream::OrderedStreamExt, interface, Interface};

use super::names;

#[derive(Debug)]
pub struct Watcher {
    tokio_rt: tokio::runtime::Handle,

    hosts: Arc<Mutex<HashSet<String>>>,
    items: Arc<Mutex<HashSet<String>>>,
}
// only used if there are no other trays
#[interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    /// RegisterStatusNotifierHost method
    async fn register_status_notifier_host(
        &self,
        service: &str,
        #[zbus(header)] hdr: zbus::MessageHeader<'_>,
        #[zbus(connection)] con: &zbus::Connection,
        #[zbus(signal_context)] ctx: zbus::SignalContext<'_>,
    ) -> zbus::fdo::Result<()> {
        let (service, _) = parse_service(service, hdr, con).await?;
        log::trace!("registering new host: {}", service);

        let added_first = {
            // scoped around locking of hosts
            let mut hosts = self.hosts.lock().await; // unwrap: mutex poisoning is okay
            if !hosts.insert(service.to_string()) {
                // we're already tracking them
                return Ok(());
            }
            hosts.len() == 1
        };

        if added_first {
            // property changed
            self.is_status_notifier_host_registered_changed(&ctx)
                .await?;
        }
        // send signal
        Watcher::status_notifier_host_registered(&ctx).await?;

        self.tokio_rt.spawn({
            let hosts = self.hosts.clone();
            let ctx = ctx.to_owned();
            let con = con.to_owned();
            async move {
                if let Err(e) = wait_for_service_exit(&con, service.as_ref().into()).await {
                    log::warn!("failed to wait for service exit: {}", e);
                }
                log::debug!("lost host: {}", service);

                let removed_last = {
                    let mut hosts = hosts.lock().await;
                    let did_remove = hosts.remove(service.as_str());
                    did_remove && hosts.is_empty()
                };

                if removed_last {
                    if let Err(e) = Watcher::is_status_notifier_host_registered_refresh(&ctx).await
                    {
                        log::warn!("failed to signal Watcher: {}", e);
                    }
                }
                if let Err(e) = Watcher::status_notifier_host_unregistered(&ctx).await {
                    log::warn!("failed to signal Watcher: {}", e);
                }
            }
        });

        Ok(())
    }

    /// RegisterStatusNotifierItem method
    async fn register_status_notifier_item(
        &self,
        service: &str,
        #[zbus(header)] hdr: zbus::MessageHeader<'_>,
        #[zbus(connection)] con: &zbus::Connection,
        #[zbus(signal_context)] ctx: zbus::SignalContext<'_>,
    ) -> zbus::fdo::Result<()> {
        let (service, objpath) = parse_service(service, hdr, con).await?;
        let service = zbus::names::BusName::Unique(service);

        let item = format!("{}{}", service, objpath);

        {
            let mut items = self.items.lock().await; // unwrap: mutex poisoning is okay
            if !items.insert(item.clone()) {
                // we're already tracking them
                log::debug!("new item: {} (duplicate)", item);
                return Ok(());
            } else {
                log::debug!("new item: {}", item);
            }
        }

        self.registered_status_notifier_items_changed(&ctx).await?;
        Watcher::status_notifier_item_registered(&ctx, item.as_ref()).await?;

        self.tokio_rt.spawn({
            let items = self.items.clone();
            let ctx = ctx.to_owned();
            let con = con.to_owned();
            async move {
                if let Err(e) = wait_for_service_exit(&con, service.as_ref()).await {
                    log::warn!("failed to wait for service exit: {}", e);
                }
                log::debug!("lost item: {}", item);

                {
                    let mut items = items.lock().await; // unwrap: mutex poisoning is okay
                    items.remove(&item);
                }

                if let Err(e) = Watcher::registered_status_notifier_items_refresh(&ctx).await {
                    log::warn!("failed to signal Watcher: {}", e);
                }
                if let Err(e) =
                    Watcher::status_notifier_item_unregistered(&ctx, item.as_ref()).await
                {
                    log::warn!("failed to signal Watcher: {}", e);
                }
            }
        });

        Ok(())
    }

    /// StatusNotifierHostRegistered signal
    #[zbus(signal)]
    async fn status_notifier_host_registered(ctx: &zbus::SignalContext<'_>) -> zbus::Result<()>;

    /// StatusNotifierHostUnregistered signal
    #[zbus(signal)]
    async fn status_notifier_host_unregistered(ctx: &zbus::SignalContext<'_>) -> zbus::Result<()>;

    /// StatusNotifierItemRegistered signal
    #[zbus(signal)]
    async fn status_notifier_item_registered(
        ctx: &zbus::SignalContext<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    /// StatusNotifierItemUnregistered signal
    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        ctx: &zbus::SignalContext<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    /// IsStatusNotifierHostRegistered property
    #[zbus(property)]
    async fn is_status_notifier_host_registered(&self) -> bool {
        !self.hosts.lock().await.is_empty()
    }

    /// ProtocolVersion property
    #[zbus(property)]
    async fn protocol_version(&self) -> i32 {
        0
    }

    /// RegisteredStatusNotifierItems property
    #[zbus(property)]
    async fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.lock().await.iter().cloned().collect()
    }
}

impl Watcher {
    pub fn new(rt: tokio::runtime::Handle) -> Self {
        Self {
            tokio_rt: rt,
            hosts: Arc::new(Mutex::new(HashSet::new())),
            items: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn attach_to(self, con: &zbus::Connection) -> zbus::Result<()> {
        // register /StatusNotifierWatcher service
        if !con.object_server().at(names::WATCHER_OBJECT, self).await? {
            return Err(zbus::Error::Failure(format!(
                "Object already exists at {} on this connection -- is StatusNotifierWatcher already running?",
                names::WATCHER_OBJECT
            )));
        }

        // try to alias self object as org.kde.StatusNotifierWatcher
        // not AllowReplacement, not ReplaceExisting, not DoNotQueue
        let flags: [zbus::fdo::RequestNameFlags; 0] = [];
        match con
            .request_name_with_flags(names::WATCHER_BUS, flags.into_iter().collect())
            .await
        {
            Ok(zbus::fdo::RequestNameReply::PrimaryOwner) => {
                log::debug!("Primary owner");
                Ok(())
            }
            Ok(_) | Err(zbus::Error::NameTaken) => {
                log::debug!("Name taken");
                Ok(())
            } // defer to existing
            Err(e) => Err(e),
        }
    }

    /// Equivalent to `is_status_notifier_host_registered_invalidate`, but without requiring
    /// `self`.
    async fn is_status_notifier_host_registered_refresh(
        ctxt: &zbus::SignalContext<'_>,
    ) -> zbus::Result<()> {
        zbus::fdo::Properties::properties_changed(
            ctxt,
            Self::name(),
            &std::collections::HashMap::new(),
            &["IsStatusNotifierHostRegistered"],
        )
        .await
    }

    /// Equivalent to `registered_status_notifier_items_invalidate`, but without requiring `self`.
    async fn registered_status_notifier_items_refresh(
        ctxt: &zbus::SignalContext<'_>,
    ) -> zbus::Result<()> {
        zbus::fdo::Properties::properties_changed(
            ctxt,
            Self::name(),
            &std::collections::HashMap::new(),
            &["RegisteredStatusNotifierItems"],
        )
        .await
    }
}

/// Decode the service name that others give to us, into the [bus
/// name](https://dbus2.github.io/zbus/concepts.html#bus-name--service-name) and the [object
/// path](https://dbus2.github.io/zbus/concepts.html#objects-and-object-paths) within the
/// connection.
///
/// The freedesktop.org specification has the format of this be just the bus name, however some
/// status items pass non-conforming values. One common one is just the object path.
async fn parse_service<'a>(
    service: &'a str,
    hdr: zbus::MessageHeader<'_>,
    con: &zbus::Connection,
) -> zbus::fdo::Result<(zbus::names::UniqueName<'static>, &'a str)> {
    if service.starts_with('/') {
        // they sent us just the object path
        if let Some(sender) = hdr.sender() {
            Ok((sender.to_owned(), service))
        } else {
            log::warn!("unknown sender for StatusNotifierItem");
            Err(zbus::fdo::Error::InvalidArgs("Unknown bus address".into()))
        }
    } else {
        // parse the bus name they gave us
        let busname: zbus::names::BusName = match service.try_into() {
            Ok(x) => x,
            Err(e) => {
                log::warn!("received invalid bus name {:?}: {}", service, e);
                return Err(zbus::fdo::Error::InvalidArgs(e.to_string()));
            }
        };

        if let zbus::names::BusName::Unique(unique) = busname {
            Ok((unique.to_owned(), names::ITEM_OBJECT))
        } else {
            // they gave us a "well-known name" like org.kde.StatusNotifierHost-81830-0, we need to
            // convert this into the actual identifier for their bus (e.g. :1.234), so that even if
            // they remove that well-known name it's fine.
            let dbus = zbus::fdo::DBusProxy::new(con).await?;
            match dbus.get_name_owner(busname).await {
                Ok(owner) => Ok((owner.into_inner(), names::ITEM_OBJECT)),
                Err(e) => {
                    log::warn!("failed to get owner of {:?}: {}", service, e);
                    Err(e)
                }
            }
        }
    }
}

/// Wait for a DBus service to disappear
async fn wait_for_service_exit(
    con: &zbus::Connection,
    service: zbus::names::BusName<'_>,
) -> zbus::fdo::Result<()> {
    let dbus = zbus::fdo::DBusProxy::new(con).await?;
    let mut owner_changes = dbus
        .receive_name_owner_changed_with_args(&[(0, &service)])
        .await?;

    if !dbus.name_has_owner(service.as_ref()).await? {
        // service has already disappeared
        return Ok(());
    }

    while let Some(sig) = owner_changes.next().await {
        let args = sig.args()?;
        if args.new_owner().is_none() {
            break;
        }
    }

    Ok(())
}
