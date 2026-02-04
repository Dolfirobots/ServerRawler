---
sidebar_position: 4
---

# `--ping`

**Short**: `-p`

**Long**: `--ping`

Performs a Server List Ping (SLP) check on a specified Minecraft server address. This command is used for quick debugging or to retrieve basic information from a single server.

:::note
The SLP protocol gathers essential server details such as version, player count, MOTD, and favicon.
:::

## Usage

```bash
ServerRawler --ping <ADDRESS>
ServerRawler -p <ADDRESS>
```

The `<ADDRESS>` should be in the format `<IP>[:PORT]`. If the port is omitted, `25565` (the default Minecraft port) will be used.

## Examples

To ping a server at a specific IP address using the default port (25565):

```bash
ServerRawler -p 192.168.1.1
```

To ping a server at a specific IP address and port:

```bash
ServerRawler -p 192.168.1.1:25567
```

Example output (simplified):

```
[INFO   ] Starting Ping for 192.168.1.1
[SUCCESS] Ping response from 192.168.1.1:
  • Version: 1.20.1 (Protocol: 764)
  • Players Online: 10/100
  • Latency: 50.23ms
  • Secure Chat: Enforced
  • Description:
  | §aWelcome §c§lto the §8Server!
  • Plain Description:
  | Welcome to the Server!
  • Favicon: data:image/png;base64,...
```