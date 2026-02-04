---
sidebar_position: 100
---

# Usage Examples

ServerRawler's flexibility allows for a wide range of advanced use cases. Here are some examples demonstrating how to combine arguments and leverage its features for more sophisticated scanning and data collection workflows.

## 1. Targeted Crawling within a Specific Network, Without Database

This example shows how to continuously crawl for Minecraft servers within a particular CIDR block, display results directly to the console, and avoid database storage. This is ideal for quick network audits or when you only need ephemeral data.

```bash
ServerRawler --crawl 100 50000 --cidr 192.168.1.0/24 --no-database
```

* `--crawl 100 50000`: Initiates crawling with 100 concurrent tasks, generating 50,000 IPs per iteration within the specified CIDR.
* `--cidr 192.168.1.0/24`: Restricts IP generation to the `192.168.1.0/24` subnet.
* `--no-database`: Ensures no data is stored persistently.

## 2. Generating an IP List

You might want to generate a large list of public IPs and then process them later or with another tool. This example generates 1 million IPs and saves them to a file.

```bash
ServerRawler --generate-ips targets.txt 1000000 --cidr 192.168.1.0/24
```

* `--generate-ips targets.txt 1000000`: Generates one million random public IP addresses and writes them to `targets.txt`.
* `--cidr 192.168.1.0/24`: Restricts IP generation to the `192.168.1.0/24` subnet.

## 3. Scanning a Custom List of Targets and Detailed Querying

If you have a curated list of potential Minecraft servers (e.g., from a previous `--generate-ips` run or an external source), you can scan them and perform a detailed query on responsive servers.

```bash
# First, generate a list (or use an existing one)
ServerRawler --generate-ips targets.txt 50000

# Then, scan the generated list and automatically query detailed info for found servers
ServerRawler --scan targets.txt
```

In the second command (`--scan`), ServerRawler will ping each IP from `targets.txt`. If a server responds to a ping, ServerRawler's internal logic will attempt to perform a query to gather more detailed information (plugins, player lists, etc.) before saving to the database (if `--no-database` is not used).

## 4. Converting a Server Favicon from an API Response

Imagine you've retrieved a server's status via another API or tool, and it returned the favicon as a Base64 string. You can use ServerRawler's utility to convert it into a usable image file.

```bash
# Assuming you have a Base64 string in a variable or file
FAVICON_BASE64="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA..."
ServerRawler --convert-image server_icon.png "$FAVICON_BASE64"
```

* `--convert-image server_icon.png "$FAVICON_BASE64"`: Takes the Base64 string and saves it as `server_icon.png`.

These examples showcase just a fraction of ServerRawler's capabilities. By understanding each argument, you can craft highly specific and efficient commands for your needs.