# Power Module

Shows the percentege, charge status and time left of a battery using UPower

## Minimal mode

Battery percentage and charging indicator(shows the name of the device on hover)

## Compact mode

Battery percentage, charging indicator and time left to 0% or 100% (when supported)

## Expanded mode

Graph of the battery charge over time

## Configuration

### Default values

- `battery`: The dbus or file path of the battery to use. If set to `""`, it will use UPower's `Display Device`. You can get this path by running `upower --enumerate`.

- `name`: The name of the battery to show on hover.

- `hide_if_missing`: If true, the activity will be removed when the device is not found( for example in the case of a phone that is plugged in), if false the activity will remain with the last known state(it will show the  `Display Device` data if the choosen device is not found when dynisland starts).

- `percentage_pango_markup`: Pango markup format string for the percentage inside the label, `{}` will be replaced with the percentage, this is useful if you want to customize the font style or color of the percentage

- `charging_color`: Battery color when it is charging.

- `normal_color`: Battery color when it is not charging.

- `low_color`: Battery color when it is low on charge (charge < 20%).

- `background_color`: Background color of the battery

- `max_duration_secs`: The default value of how much to go back in time in the graph. This can be changed on the fly by scrolling on the expanded widget( if there is no data before a certain point, that will be the new maximum duration)

- `draw_bars`: Wheter to draw grid on the expanded widget's graph

### Multiple widgets definitions

- `windows`: A map of window names to vector of configuration.

#### `windows` example

```ron
windows: {
    "": [ // list of widgets for the default window
        ( // all of these can be omitted and the default value will be used
            battery: "/sys/devices/pci0000:00/0000:00:14.0/usb1/1-2",
            name: "iPhone",
            hide_if_missing: true,
            percentage_pango_markup: "<span font_desc=\"Monospace Bold 11\" foreground=\"#FFFFFF00\">{}</span>",
            charging_color: "#00FF00CF",
            normal_color: "white",
            low_color: "#FDC400FF",
            background_color: "#E6E6E699",
            max_duration_secs: 60000,
            draw_bars: true,
        ),
    ],
    "secondary_monitor": [
        (
            battery: "", // use UPower's Display Device
            name: "Default Battery",
            hide_if_missing: false,
        ),
        (
            battery: "/org/freedesktop/UPower/devices/battery_BAT0",
            name: "Laptop Battery",
            hide_if_missing: false,
        ),
    ]
}
```
