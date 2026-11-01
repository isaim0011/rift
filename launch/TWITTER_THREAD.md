# Twitter / X Launch Thread

**Tweet 1 (Hook)**
🚀 Introducing Rift (v0.2.0): The high-performance Discord engine.

⚡ 235k raw events/sec (82.8k full-stack)
🛡️ 100% zero-unsafe Rust
🌐 Pure sans-I/O architecture
📦 Native bindings for Node.js, Bun, & Python

Measured on loopback with zstd-stream: 1.9x faster than live Node.js and 14x faster than Python.

🧵 Read the breakdown 👇

---

**Tweet 2 (The Problem)**
Running Discord bots at scale is notoriously memory heavy. In Discord.js and Discord.py, high-frequency message spikes stall the event loop and inflate heap memory.

Rift uses bounded FIFO state caching and zero-copy streaming decompression to keep latency down to microseconds.

---

**Tweet 3 (Sans-I/O Magic)**
Why sans-I/O?
Instead of mixing WebSocket sockets and business logic, Rift's protocol core is a pure state machine.

Bytes in ➡️ Actions out.

Result: 100% deterministic unit testing without fake network mocks, zero thread contention, and maximum raw speed.

---

**Tweet 4 (Polyglot Power)**
You don't need to know Rust to get Rust speeds.

Rift ships with first-class, idiomatic bindings:
🟢 Node.js / Bun (`npm install rift-discord`)
🐍 Python (`pip install rift-discord`)
🦀 Rust (`cargo add rift-discord`)

Same ergonomic API across all three runtimes!

---

**Tweet 5 (Code Preview)**
Here’s how clean TypeScript looks with Rift:

```typescript
import { RiftClient, Intents } from 'rift-discord';

const client = new RiftClient(
  process.env.DISCORD_TOKEN!,
  Intents.GUILDS | Intents.GUILD_MESSAGES
);

while (true) {
  const event = await client.nextEvent();
  if (!event) break;
  if (event.name === 'MESSAGE_CREATE' && event.data?.content === '!ping') {
    console.log(`Pong from ${event.data.author?.username}! ⚡`);
  }
}
```

---

**Tweet 6 (100% OpenAPI Coverage)**
Rift implements all 250 Discord REST endpoints with 100% coverage validated against Discord's official OpenAPI 3.1 specification.

Zero missing routes. Zero untyped payload gaps.

---

**Tweet 7 (AI Agent MCP Server)**
We also built an official Model Context Protocol (MCP) server for Rift!

Connect Claude, Cursor, or Antigravity to generate type-safe Discord bots and audit bitwise permissions.
Deploy free to Cloudflare Workers in 10 seconds:
`wrangler deploy`

---

**Tweet 8 (Links & Install)**
Rift v0.2.0 is live worldwide!

⭐ GitHub: https://github.com/isaim0011/rift
🌐 Website: https://rift-discord.dev/
🦀 Crates.io: https://crates.io/crates/rift-discord
📦 npm: https://www.npmjs.com/package/rift-discord
🐍 PyPI: https://pypi.org/project/rift-discord/

Drop a star on GitHub and build something fast today! ⚡
