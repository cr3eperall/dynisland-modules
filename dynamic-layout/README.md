# Dynamic LayoutManager

## Shown activities

It shows `config.max_activities` activities in a row. If there are more activities than that, you can scroll through them with mouse forward/backward buttons or by dragging on an activity in minimal mode.

## Active activities

It keeps maximum `config.max_active` activities in compact mode. You can put an activity in compact mode by left clicking on it.

If there are already `config.max_active` activities in compact mode, the active one that is farther from the new one will be put in minimal mode.

## Configuration

To enable this layout manager, put `"DynamicLayout"` in the layout setting

### Default values

- `auto_minimize_timout`: If an activity is in expanded or overlay mode, when the mouse leaves the widget for `auto_minimize_timeout` seconds, it will be put in compact mode.

- `max_activities`: Maximum number of activities shown.

- `max_active`: Maximum number of activities shown in compact mode.

<!-- - `activity_order`: List of activities in the order they should be shown, you can use the activity id (given by `dynisland list-activities`) or the module name. -->

- `reorder_on_add`: Will reorder the activities according to `activity_order` when a module adds a new activity.

- `reorder_on_reload`: Will reorder the activities according to `activity_order` when a the config/css is changed or `dynisland reload` is sent.

- `window_postion`: Position of the window, works like other layer shell bars.

- `window_postion.layer_shell`: Puts the window in a layer shell or a normal window.

### Multiple windows definitions

- `windows`: A map of window names to window configuration.

#### `windows` example

```ron
windows: {
    "": ( // configures the default windows, widgets defined with a non-existing window will be put here
        // all of these can be omitted and the default value will be used
        window_position: (
            layer: ("top"),
            h_anchor: ("Center"),
            v_anchor: ("start"),
            margin_x: 0,
            margin_y: 0,
            exclusive_zone: -1,
            monitor: "DP-1",
            layer_shell: true,
        ),
        auto_minimize_timeout: 5000,
        max_activities: 3,
        max_active: 3,
        reorder_on_add: true,
        reorder_on_reload: true,
        activity_order: [ // List of activities in the order they should be shown, you can use the activity id (given by `dynisland list-activities`) or the module name.
            "MusicModule",
            "ScriptModule",
        ]
    ),
    "main_montor_left": ( // creates a new window named "main_montor_left"
        window_position: ( // the values in `window_position` can be omitted and the default value will be used
            h_anchor: ("start"),
            monitor: "DP-1",
        ),
        activity_order: [
            "ClockModule",
            "MusicModule",
        ]
    ),
}
```

## Commands

to send commands to the layout manager, use `dynisland layout <command>`

- `add-css <css_class>` or `add-css [window_name] <css_class>`: Adds a css class to the activity container in that window (if no window is specified, the default one is used).
- `remove-css <css_class>` or `remove-css [window_name] <css_class>`: Removes a css class from the activity container in that window (if no window is specified, the default one is used).
- `show` or `show [window_name]`: Shows a previously hidden window (if no window is specified, the default one is used).
- `hide` or `hide [window_name]`: Hides the window with the given name, acts the same as if the window was closed (if no window is specified, the default one is used).
- `toggle` or `toggle [window_name]`: Toggles the windows visibility (if no window is specified, the default one is used).
- `help`: Shows an help message.
