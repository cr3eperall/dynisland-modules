use std::{fs, io::Read};

pub async fn get_album_art_from_url(url: &str) -> Option<Vec<u8>> {
    let vec: Vec<u8>;
    if url.starts_with("http") {
        // for some reason Cider 2.5.0 doesn't follow the mpris spec so i have to do this
        // TODO remove if they decide to fix it
        let url = url.replace("{f}", "png");
        // TODO better check for http or https
        let response = reqwest::Client::new().get(url).send().await;
        if let Ok(response) = response {
            vec = response.bytes().await.unwrap().to_vec();
        } else {
            return None;
        }
    } else if let Some(path) = url.strip_prefix("file://") {
        let mut buf = Vec::new();
        let size = fs::File::open(path).ok()?.read_to_end(&mut buf).ok()?;
        vec = buf[..size].to_vec();
    } else {
        return None;
    }
    Some(vec)
}

pub fn format_rgb_color(data: [u8; 3]) -> String {
    let (r, g, b) = (data[0], data[1], data[2]);
    format!("rgb({r}, {g}, {b})")
}

pub fn remap_num(val: u8, old_min: u8, old_max: u8, new_min: u8, new_max: u8) -> u8 {
    let val = val as u16;
    let (old_min, old_max, new_min, new_max) = (
        old_min as u16,
        old_max as u16,
        new_min as u16,
        new_max as u16,
    );
    let clamped = val.clamp(old_min, old_max);
    (new_min + ((clamped - old_min) * (new_max - new_min)) / (old_max - old_min)) as u8
}
