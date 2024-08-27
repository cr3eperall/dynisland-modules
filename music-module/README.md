# Music Module

Mpris2 client with a visualizer using cava

## Minimal mode

Album art of the current song

## Compact mode

Album art, scrolling song title and visualizer with the song's colors

## Expanded mode

Everything from compact mode plus the artist name and controls

## Configuration

- `preferred_player`: The player to use, if it's not available, it will use the first active player.

- `default_album_art_url`: The url of the default album art, it can be a local file (must start with `file://`) or a remote url (starts with `http://` or `https://`).

- `scrolling_label_speed`: Speed of the scrolling for the song name in pixels per second.

- `cava_visualizer_script`: Path to the cava script, you can copy [cava-config](cava-config) to `~/.config/dynisland/scripts` and set this to `cava -p ~/.config/dynisland/scripts/cava-config | awk '{print substr($0, 1, length($0)-1); fflush()}'`
