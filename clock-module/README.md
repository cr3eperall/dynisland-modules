# Clock Module

## Minimal mode

Analog clock with customizable colors

## Compact mode

Digital clock with animated digits, supports 24h or 12h format, uses local timezone.

## Configuration

### Default values

- `format_24h`: If true, the clock will show the time in 24h format, if false, it will show it in 12h format.

- `hour_hand_color`: Color of the hour hand.

- `minute_hand_color`: Color of the minute hand.

- `tick_color`: Color of the ticks on the analog clock.

- `circle_color`: Color of the circle in the analog clock.

### Multiple widgets definitions

- `windows`: A map of window names to vector of configuration.

#### `windows` example

```ron
windows: {
    "main_monitor": [ // list of widgets for the window named "main_monitor"
        ( // all of these can be omitted and the default value will be used
            format_24h: true,
            hour_hand_color: "white",
            minute_hand_color: "red",
            tick_color: "green",
            circle_color: "blue",
        ),
    ],
    "second_monitor": [
        (
            format_24h: false,
            circle_color: "green",
        ),
        (
            tick_color: "purple",
            circle_color: "rgb(255, 55, 87)",
        ),
    ]
}
```
