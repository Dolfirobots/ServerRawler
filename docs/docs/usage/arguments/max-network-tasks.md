---
sidebar_position: 3
---

# `--max-network-tasks`

**Long**: `--max-network-tasks`

Sets the maximum number of concurrent network tasks (e.g., pings, queries) that ServerRawler will execute at the same time. This controls the parallelism of the scanning process.

:::warning
Adjusting this value can significantly impact performance and resource usage.  
A higher value might speed up scanning but consume more network bandwidth and CPU. A lower value reduces resource consumption but will be slower.  
When you increase this value too much, server can't response correctly, so the connection will get lost, even if the server is a Minecraft Server.
:::

## Usage

```bash
ServerRawler --max-network-tasks <NUMBER>
```

### Default Value

If `--max-network-tasks` is unset, it defaults to `2000`.

## Examples

To set the maximum concurrent network tasks to 500:

```bash
ServerRawler --max-network-tasks 500
```