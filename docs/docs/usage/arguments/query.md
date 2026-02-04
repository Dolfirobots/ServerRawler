---
sidebar_position: 5
---

# `--query`

**Short**: `-q`

**Long**: `--query`

Retrieves detailed server information using the Query (UT3/GS4) protocol. This protocol provides more extensive data than a standard ping, including a list of online players, installed plugins, and the server's software.

:::info
The Query protocol is often enabled on Minecraft servers for monitoring tools and provides richer insights into server configuration and player activity.
:::

## Usage

```bash
ServerRawler --query <ADDRESS>
ServerRawler -q <ADDRESS>
```

The `<ADDRESS>` should be in the format `<IP>[:PORT]`. If the port is omitted, `25565` (the default Minecraft port) will be used.

## Examples

To query a server at a specific IP address using the default port (25565):

```bash
ServerRawler --query 192.168.1.1
```

To query a server at a specific IP address and port:

```bash
ServerRawler -q play.example.com:25567
```

Example output (simplified):

```
[INFO   ] Starting Query for play.example.com:25567
[SUCCESS] Query response from play.example.com:25567:
  • Software: Paper (1.20.1)
  • Players: 5/20
  • Online Players:
    │ Player1 (uuid-1)
    │ Player2 (uuid-2)
  • Plugins:
    │ EssentialsX (2.19.7)
    │ WorldEdit (7.2.11)
```