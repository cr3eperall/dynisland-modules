# Script Module

## Minimal mode

Shows the image or icon from `config.minimal_image`

## Compact mode

The output of the script in `config.exec`

## Configuration

- `exec`: The script to run and show in compact mode.

- `minimal_image`: The image or icon to show in minimal mode. It can be a path ( begins with `file://`), a url (begins with `http://` or `https://`) or a gtk4 icon name (no prefix, you can see a collection of icons using `gtk4-icon-browser`).

- `scrolling`: If true, the output of `exec` will scroll if it's longer than `max_width` in pixels, otherwise it will be ellipsized if longer than `max_width` in characters.

- `max_width`: The maximum width of the widget in pixels if `scrolling` is true or in characters if `scrolling` is false. You can configure the minimum width in css.

- `scrolling_speed`: The speed of the scrolling in pixels per second.
