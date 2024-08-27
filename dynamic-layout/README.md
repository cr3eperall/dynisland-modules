# Dynamic LayoutManager

## Shown activities

It shows `config.max_activities` activities in a row. If there are more activities than that, you can scroll through them with mouse forward/backward buttons or by dragging on an activity in minimal mode.

## Active activities

It keeps maximum `config.max_active` activities in compact mode. You can put an activity in compact mode by left clicking on it.

If there are already `config.max_active` activities in compact mode, the active one that is farther from the new one will be put in minimal mode.

## Configuration

### To enable this layout manager, put `"DynamicLayout"` in the layout setting

- `auto_minimize_timout`: If an activity is in expanded or overlay mode, when the mouse leaves the widget for `auto_minimize_timeout` seconds, it will be put in compact mode.

- `max_activities`: Maximum number of activities shown.

- `max_active`: Maximum number of activities shown in compact mode.

- `activity_order`: List of activities in the order they should be shown, you can use the activity id (given by `dynisland list-activities`) or the module name.

- `reorder_on_add`: Will reorder the activities according to `activity_order` when a module adds a new activity.

- `reorder_on_reload`: Will reorder the activities according to `activity_order` when a the config/css is changed or `dynisland reload` is sent.

- `window_postion`: Position of the window, works like other layer shell bars.

- `window_postion.layer_shell`: Puts the window in a layer shell or a normal window.
