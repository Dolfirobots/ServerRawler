---
sidebar_position: 9
---

# `--cidr`

**Long**: `--cidr`

Configures an IP range in CIDR (Classless Inter-Domain Routing) format to restrict the scope of IP generation (`--generate-ips`) or crawling (`--crawl`) operations. This allows focusing on specific networks or segments.

:::info
CIDR notation specifies an IP address and a prefix length, indicating the number of bits in the IP address that represent the network prefix. For example, `192.168.1.0/24` covers all IP addresses from `192.168.1.0` to `192.168.1.255`.
:::

## Usage

```bash
ServerRawler --cidr <IP_RANGE>
```

*   `<IP_RANGE>`: The IP range in CIDR format (e.g., `192.168.1.0/24`, `10.0.0.0/8`).

## Examples

To generate random IPs exclusively within the `172.16.0.0/16` range:

```bash
ServerRawler --generate-ips private_network.txt 1000 --cidr 172.16.0.0/16
```

To start a crawling loop that only targets servers within the `8.8.8.0/29` range:

```bash
ServerRawler --crawl --cidr 8.8.8.0/29
```