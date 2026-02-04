---
sidebar_position: 1
---

# `--log`

**Short**: `-s`

Sets the minimum logging level threshold for console output. Messages with a level lower than the specified threshold will not be displayed.

:::info
This argument controls the verbosity of ServerRawler's output, helping you focus on critical information or dive into detailed debugging.
:::

## Usage

```bash
ServerRawler --log <LEVEL>
ServerRawler -s <LEVEL>
```

### Available Log Levels

The following log levels are available, ordered from least to most verbose:

*   **`Debug`**: Highly detailed messages for debugging purposes.
*   **`Info`**: General information messages about the program's progress. (Default)
*   **`Success`**: Indicates successful operations or discoveries.
*   **`Warning`**: Highlights potential issues or non-critical errors.
*   **`Error`**: Reports errors that prevent specific operations but don't halt the program.
*   **`Critical`**: Severe errors that might lead to program termination or major malfunction.

## Examples

To display only `Info`, `Success`, `Warning`, `Error`, and `Critical` messages (default behavior):

```bash
ServerRawler --log info
```

To display all messages, including `Debug` (most verbose):

```bash
ServerRawler -s debug
```

To only display `Error` and `Critical` messages:

```bash
ServerRawler --log error
```