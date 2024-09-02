# Example implementation of a dynisland module

## Configuration

### Default values

- `duration`: The delay between the rolling characters in milliseconds.

The rest of the config is just some random types to show how the config works.

### Multiple widgets definitions

- `windows`: A map of window names to vector of configuration.

#### `windows` example

```ron
windows: {
    "main_monitor": [ // list of widgets for the window named "main_monitor"
        ( // all of these can be omitted and the default value will be used
            duration: 100,
        ),
    ],
    "second_monitor": [
        (
            duration: 200,
        ),
        (
            duration: 300,
        ),
        (
            duration: 400,
        ),
    ]
}
```
