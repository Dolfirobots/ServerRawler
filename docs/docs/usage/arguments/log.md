---
title: --log - Argument
---

# `--log` - Argument

- **Short**: `-l`

## Description

Sets the minimum logging level threshold for console output. Messages with a level lower than the specified threshold will not be displayed.
For example, if you set the log level to `Warning`, only messages with levels `Warning`, `Error`, and `Critical` will be shown, while `Debug`, `Info`, and `Success` messages will be hidden.

:::info[Hint]
The plain messages currently are visible on every log level.
:::

## Usage

```bash
ServerRawler --log <LEVEL>
ServerRawler -l <LEVEL>
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
./ServerRawler --log info
```

To display all messages, including `Debug` (most verbose):

```bash
./ServerRawler -l debug
```

To only display `Error` and `Critical` messages:

```bash
./ServerRawler --log error
```