---
sidebar_position: 6
---

# `--join`

**Short**: `-j`

**Long**: `--join`

Simulates a player login to a Minecraft server to check authentication and whitelist status. This feature attempts to join the server and captures any kick messages, which can indicate if a server is online-mode (requires Mojang authentication), cracked (offline-mode), or has a whitelist enabled.

:::warning
This feature is currently under active development. While it provides basic authentication and whitelist status, its behavior might evolve, and detailed kick reasons might not always be perfectly parsed.
:::

## Usage

```bash
ServerRawler --join <ADDRESS>
ServerRawler -j <ADDRESS>
```

The `<ADDRESS>` should be in the format `<IP>[:PORT]`. If the port is omitted, `25565` (the default Minecraft port) will be used.

## Examples

To perform a join check on a server at a specific IP address:

```bash
ServerRawler --join 192.168.1.1
```

To perform a join check on a server at a specific IP address and port:

```bash
ServerRawler -j example.org:25565
```

Example output (simplified):

```
[INFO   ] Starting Join-Check for example.org:25565
[WARNING] Please note this feature is in development
[SUCCESS] Join-Check completed for example.org:25565:
  • Auth-Type: Online-Mode (Premium Only)
  • Whitelist: Enabled
  • Kick-Reason:
    │ You are not whitelisted on this server!
```