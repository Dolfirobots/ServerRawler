---
sidebar_position: 1
---

# Welcome to ServerRawler

A blazing fast Minecraft Server Scanner that can be used to monitor server or find new server.

:::info
ServerRawler is coded in Rust, so it has blazing-fast execution speeds and optimal resource utilization, making it possible to scan millions of IPs in a amazing time.
:::

## ✨ Features

* **⚡ High-Performance:** Using the speed from Rust, ServerRawler can scan thousands of servers at the same time.
* **📡 Multi-Protocol Support:**
  * **Minecraft Ping Protocol:** Quickly gathers fundamental server data like MOTD, player counts, and version information.
  * **Minecraft Query Protocol:** More details, including player lists, plugins, and server software, for servers that support it.
  * **Join Protocol (Experimental):** Simulates player logins to check authentication requirements and whitelist status.
* **📋 PostgreSQL:** Fetched data will saved in a PostgreSQL.
* **🔧 Configurable:** Designed to be easy to configure and install.

:::tip
ServerRawler is continuously in development! We're on it to add more features and refining existing ones to provide the most robust and versatile Minecraft server scanning experience possible.
:::

## Getting Started on Your Journey

Ready to explore the Minecraft server? Dive into our guides to set up ServerRawler and begin your first scan.

* **[Installation Guide](./getting-started/installation)**: Learn how to get ServerRawler up and running on your system.
* **[Configuration Overview](./getting-started/configuration)**: Understand how to configure your database and customize the parameters.
* **[Usage Documentation](./usage)**: Explore all command-line arguments and usage examples foe ServerRawler.