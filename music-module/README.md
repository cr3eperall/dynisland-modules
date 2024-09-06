# Music Module

Mpris2 client with a visualizer using cava

## Minimal mode

Album art of the current song

## Compact mode

Album art, scrolling song title and visualizer with the song's colors

## Expanded mode

Everything from compact mode plus the artist name and controls

## Configuration

### Default values

- `preferred_player`: The player to use, if it's not available, it will use the first active player it finds. If it's set to `""`, it will dynamically choose the active player.

- `default_album_art_url`: The url of the default album art, it can be a local file (must start with `file://`) or a remote url (starts with `http://` or `https://`).

- `scrolling_label_speed`: Speed of the scrolling for the song name in pixels per second.

- `cava_visualizer_script`: Path to the cava script, you can copy [cava-config](cava-config) to `~/.config/dynisland/scripts` and set this to `cava -p ~/.config/dynisland/scripts/cava-config | awk '{print substr($0, 1, length($0)-1); fflush()}'`

- `use_fallback_player`: If the preferred player is not available, use the next available player, if it's set to `false`, it will remove the widget if the preferred one is not available. (if the preferred player is `""`, this will be ignored)

### Multiple widgets definitions

- `windows`: A map of window names to vector of configuration.

#### `windows` example

```ron
windows: {
    "": [ // list of widgets for the default window
        ( // all of these can be omitted and the default value will be used
            preferred_player: "firefox",
            use_fallback_player: true,
            default_album_art_url: "file:///path/to/image.png",
            scrolling_label_speed: 30,
            cava_visualizer_script: "cava -p ~/.config/dynisland/scripts/cava-config | awk '{print substr($0, 1, length($0)-1); fflush()}'"
        ),
    ],
    "secondary_monitor": [
        (
            preferred_player: "",
        ),
        (
            preferred_player: "spotify",
            use_fallback_player: false,
        ),
    ]
}
```
