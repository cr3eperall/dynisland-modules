use std::sync::Arc;

use dynisland_core::{
    abi::{
        glib::{self, subclass::types::ObjectSubclassIsExt},
        log,
    },
    base_module::ProducerRuntime,
};
use tokio::sync::Mutex;
use zbus::export::ordered_stream::OrderedStreamExt;

use crate::{
    module::SystrayModule,
    status_notifier::{self, item::Item, menu::Menu},
    widget::{expanded::Expanded, status_notifier_widgets::menu_item::MenuItemAction},
};

pub(crate) fn start_item_menu_action_task(
    rt: &ProducerRuntime,
    module: &SystrayModule,
    expanded_action_rx: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<(String, MenuItemAction)>>>,
) {
    rt.handle().spawn({
        let items = module.items.clone();
        async move {
            while let Some((id, action)) = expanded_action_rx.lock().await.recv().await {
                let menu = {
                    let mut lock = items.lock().await;
                    let item = match lock.get_mut(&id) {
                        Some(it) => it,
                        None => continue,
                    };
                    if let Some(menu) = item.1.clone() {
                        menu
                    } else {
                        match get_menu(&item.0).await {
                            Some(menu) => {
                                item.1 = Some(menu.clone());
                                menu
                            }
                            None => continue,
                        }
                    }
                };
                match action {
                    MenuItemAction::Clicked(id) => {
                        if menu
                            .event(id, status_notifier::menu::Event::Clicked, None)
                            .await
                            .is_err()
                        {
                            log::warn!("failed to send click event to menu");
                        }
                    }
                    MenuItemAction::Hovered(id) => {
                        if menu
                            .event(id, status_notifier::menu::Event::Hovered, None)
                            .await
                            .is_err()
                        {
                            log::trace!("failed to send hover event to menu");
                        }
                    }
                    MenuItemAction::OpenMenu(id) => {
                        if menu.about_to_show(id).await.is_err() {
                            log::trace!("failed to send about_to_show event to menu");
                        }
                    }
                    _ => {
                        log::warn!(
                            "unexpected action: {:?}, should get filtered by MenuPage",
                            action
                        );
                    }
                }
            }
        }
    });
}
pub(crate) async fn start_item_menu_update_task(expanded: &Expanded, menu: Menu, item_id: &String) {
    let old_cleanup = expanded.imp().cleanup_tx.borrow().clone();
    if let Some(old_cleanup) = old_cleanup {
        let _ = old_cleanup.send(());
    }
    let (cleanup_tx, cleanup_rx) = tokio::sync::broadcast::channel::<()>(1);

    let layout_updated = menu.dm.receive_layout_updated().await;
    if let Ok(mut layout_updated) = layout_updated {
        glib::MainContext::default().spawn_local({
            // TODO could use normal runtime
            let mut cleanup = cleanup_rx.resubscribe();
            let expanded = expanded.clone();
            let menu = menu.clone();
            let item_id = item_id.clone();
            async move {
                while let Some(_) = tokio::select! {
                    res = layout_updated.next() => {
                        if let Some(update) = res {
                            if let Ok(args)=update.args() {
                                let mut layout = expanded.imp().layout.borrow().clone();
                                if layout.update_child(&menu.dm, args.parent).await.is_err(){
                                    log::warn!("failed to update layout");
                                };
                                let current_path = expanded.imp().current_path.borrow().clone();
                                expanded.set_layout(layout, Some(current_path), item_id.clone());
                            }
                        }
                        Some(())
                    }
                    _ = cleanup.recv() => {
                        None
                    }
                } {}
            }
        });
    }

    let item_properties_updated = menu.dm.receive_items_properties_updated().await;
    if let Ok(mut item_properties_updated) = item_properties_updated {
        glib::MainContext::default().spawn_local({
            let mut cleanup = cleanup_rx.resubscribe();
            let expanded = expanded.clone();
            let item_id = item_id.clone();
            async move {
                while let Some(_) = tokio::select! {
                    res = item_properties_updated.next() => {
                        if let Some(update) = res {
                            if let Ok(args)=update.args() {
                                let mut layout = expanded.imp().layout.borrow().clone();
                                if layout.update_props(&args).await.is_err(){
                                    log::warn!("failed to update layout");
                                };
                                let current_path = expanded.imp().current_path.borrow().clone();
                                expanded.set_layout(layout, Some(current_path), item_id.clone());
                            }
                        }
                        Some(())
                    }
                    _ = cleanup.recv() => {
                        None
                    }
                } {}
            }
        });
    }
    expanded.imp().cleanup_tx.replace(Some(cleanup_tx));
}

pub(crate) async fn get_menu(item: &Item) -> Option<Menu> {
    let menu = item.menu().await.ok();
    let menu = if let Some(menu_path) = menu {
        Menu::from_address(
            item.sni.inner().connection(),
            item.sni.inner().destination(),
            menu_path,
        )
        .await
        .ok()
    } else {
        None
    };
    menu
}
