---
title: --config - Argument
---

# `--config` - Argument

- **Short**: `-c`  
- **Long**: `--config`

## Description

Specifies the path to custom configuration files for ServerRawler. This allows you to use your own configuration settings instead of the config/ folder in the root directory of your project. (Where you run ServerRawler from)

:::warning[Important]
The path you specify with the `--config` argument is the directory where the folder `config` is located, not the path to the configuration files itself.
For example, if your configuration files are located in `path/to/your/custom_config_folder/config`, you should specify `--config path/to/your/custom_config_folder` as the argument.
:::

## Usage

```bash
ServerRawler --config <PATH_TO_CONFIG_FOLDER>
ServerRawler -c <PATH_TO_CONFIG_FOLDER>
```

- `<PATH_TO_CONFIG_FOLDER>`: The absolute or relative path to your custom configuration folder.


## Examples

To run ServerRawler using a custom configuration folder located at `my_custom_config_folder` in the current directory:

```bash
./ServerRawler --config my_custom_config_folder
```

To run ServerRawler with a configuration folder located at a specific absolute path:

```bash
./ServerRawler -c /etc/serverrawler/production_config_folder
```