# Systray Module

System tray using the [StatusNotifierItem](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierItem/) and `com.canonical.dbusmenu` protocols.

## Minimal mode

Number of items in the tray

## Compact mode

List of items(icons) in the tray

## Expanded mode

The menu of the item that was clicked if it has one.

## Configuration

### Default values

- `menu_height_mode`: Controls the animation when opening a submenu. It can be:
  - `"max"`: The submenu height is the max between the height of the submenu and the height of the parent
  - `"current"`: The submenu is as tall as it needs to be to show all of its items
  - `"2-step"`: The submenu is as tall as it needs to be but the height is changed only after the parent is fully hidden. This is the default.

### Multiple widgets definitions

- `windows`: A map of window names to vector of configuration.

#### `windows` example

```ron
windows: {
    "": [ // list of widgets for the default window
        ( // all of these can be omitted and the default value will be used
            menu_height_mode: "max",
        ),
    ],
    "secondary_monitor": [
        (
            menu_height_mode: "2-step",
        ),
        {}, // To use all the default values, curly braces are needed, due to how ron works
    ]
}
```
