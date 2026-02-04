---
sidebar_position: 10
---

# `--crawl`

**Long**: `--crawl`

Starts a continuous crawling loop to discover Minecraft servers. The crawler generates IP addresses (either randomly or within a specified CIDR range) and attempts to ping them. Discovered servers are then processed and, if enabled, saved to the database.

:::info
This is the primary command for actively discovering new servers. It runs indefinitely until manually stopped (e.g., by pressing `Ctrl+C`).
:::

## Usage

```bash
ServerRawler --crawl [MAX_TASKS] [AMOUNT_OF_IPS]
```

* `[MAX_TASKS]`: (Optional) The maximum number of concurrent network tasks (pings) to run simultaneously. If omitted, it defaults to `2000`. This is similar to `--max-network-tasks` but specific to the crawl operation.
* `[AMOUNT_OF_IPS]`: (Optional) The number of IP addresses to generate and attempt to scan in each iteration of the crawling loop. If omitted, it defaults to `1,000,000`.

## Examples

To start crawling with default settings (2000 max tasks, 1,000,000 IPs per iteration):

```bash
ServerRawler --crawl
```

To start crawling with 500 concurrent tasks and scanning 50,000 IPs per iteration:

```bash
ServerRawler --crawl 500 50000
```

To crawl a specific CIDR range with custom task and IP generation limits:

```bash
ServerRawler --crawl 1000 100000 --cidr 10.0.0.0/8
```

To crawl without saving data to the database:

```bash
ServerRawler --crawl --no-database
```