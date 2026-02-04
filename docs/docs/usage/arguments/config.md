---
sidebar_position: 12
---

# `--config`

**Short**: `-c`

**Long**: `--config`

Specifies the path to a custom configuration file (e.g., `config.toml`). This allows users to manage settings such as database connections and crawler parameters separately from environment variables or default settings.

:::info
Using a dedicated configuration file is recommended for managing complex deployments or specific operational profiles.
:::

## Usage

```bash
ServerRawler --config <PATH_TO_CONFIG_FILE>
ServerRawler -c <PATH_TO_CONFIG_FILE>
```

* `<PATH_TO_CONFIG_FILE>`: The absolute or relative path to your `config.toml` file.

## Examples

To run ServerRawler using a configuration file named `my_custom_config.toml` located in the current directory:

```bash
ServerRawler --config my_custom_config.toml
```

To run ServerRawler with a configuration file located at a specific absolute path:

```bash
ServerRawler -c /etc/serverrawler/production.toml
```