use std::sync::Arc;

use dynisland_core::{abi::glib, cast_dyn_any, dynamic_property::DynamicPropertyAny};
use tokio::sync::Mutex;
use zbus::export::ordered_stream::OrderedStreamExt;

use crate::{
    status_notifier::item::Item,
    widget::compact::{Compact, ItemData},
};

pub(crate) fn start_remove_item_task(
    remove_item_rx: &tokio::sync::broadcast::Receiver<String>,
    minimal_count: Arc<Mutex<DynamicPropertyAny>>,
    compact: &Compact,
) {
    glib::MainContext::default().spawn_local({
        let mut remove_item_rx = remove_item_rx.resubscribe();
        let compact = compact.clone();
        async move {
            while let Ok(item_id) = remove_item_rx.recv().await {
                if compact.remove_item(&item_id) {
                    let mut minimal_count = minimal_count.lock().await;
                    let count = *cast_dyn_any!(minimal_count.get(), i32).unwrap();
                    minimal_count.set(count - 1).unwrap();
                }
            }
        }
    });
}

pub(crate) fn start_add_item_task(
    main_context_cleanup_rx: &tokio::sync::broadcast::Receiver<()>,
    add_item_rx: &tokio::sync::broadcast::Receiver<(String, Item)>,
    minimal_count: Arc<Mutex<DynamicPropertyAny>>,
    compact: &Compact,
) {
    glib::MainContext::default().spawn_local({
        let mut add_item_rx = add_item_rx.resubscribe();
        let compact = compact.clone();
        let main_context_cleanup_rx = main_context_cleanup_rx.resubscribe();
        async move {
            while let Ok(item) = add_item_rx.recv().await {
                let (item_id, item) = item;
                let tooltip = match item.tooltip().await {
                    Ok((_icon_name, _icon_data, title, description)) => {
                        if !description.is_empty() {
                            format!("{}\n{}", title, description)
                        } else {
                            title
                        }
                    }
                    Err(_) => item.title().await.unwrap_or("".to_string()),
                };
                // getting the icon takes a while, so we can parallelize it
                glib::MainContext::default().spawn_local({
                    let compact = compact.clone();
                    let item = item.clone();
                    let item_id = item_id.clone();
                    let minimal_count = minimal_count.clone();
                    async move {
                        // TODO check if scale should always be 1
                        let icon = item.icon(30, 1).await;
                        let attention_icon = item.attention_icon(30, 1).await;
                        let overlay_icon = item.overlay_icon(30, 1).await;
                        let data = ItemData {
                            status: item.status().await.unwrap(),
                            icon,
                            attention_icon,
                            overlay_icon,
                            tooltip,
                        };
                        if compact.insert_item(&item_id, data) {
                            let mut minimal_count = minimal_count.lock().await;
                            let count = *cast_dyn_any!(minimal_count.get(), i32).unwrap();
                            minimal_count.set(count + 1).unwrap();
                        }
                    }
                });

                start_item_updater(&item, &main_context_cleanup_rx, &compact, item_id).await;
            }
        }
    });
}

async fn start_item_updater(
    item: &Item,
    main_context_cleanup_rx: &tokio::sync::broadcast::Receiver<()>,
    compact: &Compact,
    item_id: String,
) {
    let new_icon = item.sni.receive_new_icon().await;
    if let Ok(mut new_icon) = new_icon {
        glib::MainContext::default().spawn_local({
            let mut cleanup = main_context_cleanup_rx.resubscribe();
            let compact = compact.clone();
            let item = item.clone();
            let item_id = item_id.clone();
            async move {
                while let Some(_) = tokio::select! {
                    _ = new_icon.next() => {
                        // for some reason item properties are stuck on the same value, so a new connection is created
                        let new_item = Item::from_address(item.sni.inner().connection(), &item_id).await.unwrap_or(item.clone());
                        let icon = new_item.icon(30, 1).await;
                        compact.update_item_icon(&item_id, icon);
                        Some(())
                    }
                    _ = cleanup.recv() => {
                        None
                    }
                } {}
            }
        });
    }
    let new_icon_attention = item.sni.receive_new_attention_icon().await;
    if let Ok(mut new_icon_attention) = new_icon_attention {
        glib::MainContext::default().spawn_local({
            let mut cleanup = main_context_cleanup_rx.resubscribe();
            let compact = compact.clone();
            let item = item.clone();
            let item_id = item_id.clone();
            async move {
                while let Some(_) = tokio::select! {
                    _ = new_icon_attention.next() => {
                        // for some reason item properties are stuck on the same value, so a new connection is created
                        let new_item = Item::from_address(item.sni.inner().connection(), &item_id).await.unwrap_or(item.clone());
                        compact.update_item_attention_icon(&item_id, new_item.attention_icon(30, 1).await);
                        Some(())
                    }
                    _ = cleanup.recv() => {
                        None
                    }
                } {}
            }
        });
    }
    let new_icon_overlay = item.sni.receive_new_overlay_icon().await;
    if let Ok(mut new_icon_overlay) = new_icon_overlay {
        glib::MainContext::default().spawn_local({
            let mut cleanup = main_context_cleanup_rx.resubscribe();
            let compact = compact.clone();
            let item = item.clone();
            let item_id = item_id.clone();
            async move {
                while let Some(_) = tokio::select! {
                    _ = new_icon_overlay.next() => {
                        // for some reason item properties are stuck on the same value, so a new connection is created
                        let new_item = Item::from_address(item.sni.inner().connection(), &item_id).await.unwrap_or(item.clone());
                        compact.update_item_overlay_icon(
                            &item_id,
                            new_item.overlay_icon(30, 1).await,
                        );
                        Some(())
                    }
                    _ = cleanup.recv() => {
                        None
                    }
                } {}
            }
        });
    }
    let new_tooltip = item.sni.receive_new_tool_tip().await;
    if let Ok(mut new_tooltip) = new_tooltip {
        glib::MainContext::default().spawn_local({
            let mut cleanup = main_context_cleanup_rx.resubscribe();
            let compact = compact.clone();
            let item = item.clone();
            let item_id = item_id.clone();
            async move {
                while let Some(_) = tokio::select! {
                    _ = new_tooltip.next() => {
                        // for some reason item properties are stuck on the same value, so a new connection is created
                        let new_item = Item::from_address(item.sni.inner().connection(), &item_id).await.unwrap_or(item.clone());
                        let tooltip = match new_item.tooltip().await {
                            Ok((_icon_name, _icon_data, title, description)) => {
                                if !description.is_empty(){
                                    format!("{}\n{}", title, description)
                                } else {
                                    title
                                }
                            }
                            Err(_) => new_item.title().await.unwrap_or("".to_string()),
                        };
                        compact.update_item_tooltip(&item_id, &tooltip);
                        Some(())
                    }
                    _ = cleanup.recv() => {
                        None
                    }
                } {}
            }
        });
    }
    let new_status = item.sni.receive_new_status().await;
    if let Ok(mut new_status) = new_status {
        glib::MainContext::default().spawn_local({
            let mut cleanup = main_context_cleanup_rx.resubscribe();
            let compact = compact.clone();
            let item = item.clone();
            let item_id = item_id.clone();
            async move {
                while let Some(_) = tokio::select! {
                    _ = new_status.next()=> {
                        // for some reason item properties are stuck on the same value, so a new connection is created
                        let new_item = Item::from_address(item.sni.inner().connection(), &item_id).await.unwrap_or(item.clone());
                        compact
                            .update_item_status(&item_id, new_item.status().await.unwrap());
                        Some(())
                    }
                    _ = cleanup.recv() => {
                        None
                    }
                } {}
            }
        });
    }
    let new_title = item.sni.receive_new_title().await;
    if let Ok(mut new_title) = new_title {
        glib::MainContext::default().spawn_local({
            let mut cleanup = main_context_cleanup_rx.resubscribe();
            let compact = compact.clone();
            let item = item.clone();
            let item_id = item_id.clone();
            async move {
                while let Some(_) = tokio::select! {
                    _ = new_title.next() => {
                        // for some reason item properties are stuck on the same value, so a new connection is created
                        let new_item = Item::from_address(item.sni.inner().connection(), &item_id).await.unwrap_or(item.clone());
                        let tooltip = match new_item.tooltip().await {
                            Ok((_icon_name, _icon_data, title, description)) => {
                                if !description.is_empty(){
                                    format!("{}\n{}", title, description)
                                } else {
                                    title
                                }
                            }
                            Err(_) => new_item.title().await.unwrap_or("".to_string()),
                        };
                        compact.update_item_tooltip(&item_id, &tooltip);
                        Some(())
                    }
                    _ = cleanup.recv() => {
                        None
                    }
                } {}
            }
        });
    }
}
