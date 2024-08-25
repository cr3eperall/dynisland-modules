use std::path::PathBuf;

use anyhow::{anyhow, Result};
use gdk::{gdk_pixbuf::Pixbuf, gio::MemoryInputStream};
use glib::Bytes;

#[derive(Debug, Clone)]
pub enum ImageType {
    Texture(gdk::Texture),
    File(PathBuf),
    Icon(String),
}

pub async fn get_image_from_url(url: &str) -> Result<ImageType> {
    if url.starts_with("http") {
        // from url
        // TODO better check for http or https
        let vec = reqwest::Client::new()
            .get(url)
            .send()
            .await?
            .bytes()
            .await?
            .to_vec();
        let data = Bytes::from(&vec);
        let mut pixbuf = Pixbuf::from_stream(
            &MemoryInputStream::from_bytes(&data),
            None::<&gtk::gio::Cancellable>,
        )
        .ok();
        if pixbuf.is_none() {
            pixbuf = Pixbuf::new(gdk::gdk_pixbuf::Colorspace::Rgb, true, 8, 10, 10);
        }
        let texture = gdk::Texture::for_pixbuf(&pixbuf.unwrap());
        Ok(ImageType::Texture(texture))
    } else if let Some(path) = url.strip_prefix("file://") {
        // from file
        let path = PathBuf::from(path);
        if path.exists() {
            Ok(ImageType::File(path))
        } else {
            Err(anyhow!("file not found: {path:?}"))
        }
    } else {
        // from gtk-icon
        Ok(ImageType::Icon(url.to_string()))
    }
}
