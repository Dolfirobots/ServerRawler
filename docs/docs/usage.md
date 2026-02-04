---
sidebar_position: 3
---

# Usage Overview

ServerRawler is a powerful tool for scanning Minecraft servers, designed for efficiency and flexibility. This section provides a comprehensive guide on how to operate ServerRawler, from starting the crawler to understanding its command-line arguments and interpreting its output.

## Starting ServerRawler

Once ServerRawler is installed and configured, you can initiate its operations. Navigate to the project's root directory in your terminal.

If you prefer to use a custom configuration file, look at the [--config](./usage/arguments/scan) argument.

## Command Line Arguments

ServerRawler's behavior can be customized using a variety of command-line arguments. Each argument provides granular control over logging, scanning, IP generation, and more. Click on any argument below for detailed information and examples.

:::tip
For a quick overview of all available commands directly in your terminal, run `ServerRawler --help`.
:::

### General Arguments

* [**`--log`**](./usage/arguments/log): Set the threshold for console output.
* [**`--no-database`**](./usage/arguments/no-database): Prevent data from being saved to the database.
* [**`--max-network-tasks`**](./usage/arguments/max-network-tasks): Define the maximum concurrent network operations.
* [**`--config`**](./usage/arguments/config): Specify a custom configuration file.

### Utility & Debugging Arguments

* [**`--ping`**](./usage/arguments/ping): Perform a Server List Ping (SLP) check.
* [**`--query`**](./usage/arguments/query): Retrieve detailed server info via Query protocol.
* [**`--join`**](./usage/arguments/join): Simulate a player login for authentication/whitelist checks.
* [**`--convert-image`**](./usage/arguments/convert-image): Convert Base64 strings to image files.
* [**`--generate-ips`**](./usage/arguments/generate-ips): Generate random IPv4 addresses and save them to a file.
* [**`--cidr`**](./usage/arguments/cidr): Configure an IP range for generation or scanning.

### Scanning & Crawling Arguments

* [**`--crawl`**](./usage/arguments/crawl): Start a continuous crawling loop for discovering servers.
* [**`--scan`**](./usage/arguments/scan): Scan IP addresses from a text file.

## Output

ServerRawler provides real-time feedback through console logs, indicating its progress, any discovered servers, and errors encountered.

* Successfully discovered server data, if database saving is enabled, will be stored in your configured PostgreSQL database.
* Logs are color-coded to easily distinguish between information, warnings, errors, and successes.

## Usage Examples

Check out the [Usage Examples](./usage/examples) section for advanced workflows and command combinations to maximize your ServerRawler experience.