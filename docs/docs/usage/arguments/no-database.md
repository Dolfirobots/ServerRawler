---
sidebar_position: 2
---

# `--no-database`

**Long**: `--no-database`

When this flag is present, ServerRawler will operate without attempting to connect to or save data to the database. This is useful for testing, quick scans where persistence is not required, or environments where a database is not available.

:::tip
Using `--no-database` can reduce overhead if you only need real-time console output or temporary results without storing them.
:::

## Usage

```bash
ServerRawler --no-database
```

## Examples

To run ServerRawler and display results directly to the console without database interaction:

```bash
ServerRawler --no-database --scan ips.txt
```