# rift-discord

High-performance Node.js and Bun bindings for the **Rift** Discord engine.

## Installation

```bash
npm install rift-discord
# or
bun add rift-discord
```

## Quickstart

```javascript
const { RiftClient, Intents, Snowflake, Permissions } = require("rift-discord");

const client = new RiftClient({
  token: process.env.DISCORD_TOKEN,
  intents: Intents.GUILDS | Intents.GUILD_MESSAGES | Intents.MESSAGE_CONTENT
});

client.on("READY", (data) => {
  console.log(`Connected as ${data.user.username}#${data.user.discriminator}!`);
});

client.on("MESSAGE_CREATE", async (message) => {
  if (message.content === "!ping") {
    console.log("Ping received from:", Snowflake.timestamp(message.id));
  }
});

client.connect();
```

## Features

- **Microsecond Latency**: Built on top of Rift's zero-unsafe, sans-I/O Rust architecture.
- **Bitwise Permissions**: Full decoding and encoding of all 41 Discord permission bits.
- **Snowflake Math**: Fast extraction of timestamps, worker IDs, and process IDs from 64-bit Discord IDs.
- **Ed25519 Interaction Verification**: Cryptographic validation for Discord HTTP Interactions.
