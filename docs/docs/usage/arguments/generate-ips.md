---
sidebar_position: 8
---

# `--generate-ips`

**Long**: `--generate-ips`

Generates a specified amount of random IPv4 addresses and saves them to a text file. This is useful for creating custom lists of IPs to scan. The generated IPs can be public or restricted to a specific CIDR range using the `--cidr` argument.

## Usage

```bash
ServerRawler --generate-ips <FILE_PATH> [AMOUNT]
```

* `<FILE_PATH>`: The path to the file where the generated IP addresses will be saved, one per line.
* `[AMOUNT]`: (Optional) The number of IPv4 addresses to generate. If omitted, it defaults to `100,000`.

:::tip
Combine with `--cidr` to generate IPs within a specific range.
:::

## Examples

To generate 100,000 random public IP addresses and save them to `ips.txt`:

```bash
ServerRawler --generate-ips ips.txt
```

To generate 50,000 random public IP addresses and save them to `my_ips.txt`:

```bash
ServerRawler --generate-ips my_ips.txt 50000
```

To generate 10,000 IP addresses within a specific CIDR range (e.g., `192.168.1.0/24`):

```bash
ServerRawler --generate-ips cidr_ips.txt 10000 --cidr 192.168.1.0/24
```