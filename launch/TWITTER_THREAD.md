# Twitter / X Launch Thread

**Tweet 1 (Hook)**
🚀 Introducing Rift (v0.1.0): The high-performance Discord engine.

⚡ 820,000 events/sec throughput
🛡️ 100% zero-unsafe Rust
🌐 Pure sans-I/O architecture
📦 Native bindings for Node.js, Bun, & Python

It’s 10x faster than Discord.js with 1/10th the memory.

🧵 Read the breakdown 👇

---

**Tweet 2 (The Problem)**
Running Discord bots at scale is notoriously memory heavy. In Discord.js and Discord.py, caching 100k guilds easily burns 400MB - 1GB+ of RAM, and high-frequency message spikes stall the event loop.

Rift caches the same 100k guilds in just 42 MB.

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
import { Client, GatewayIntents } from 'rift-discord';

const client = new Client({
  token: process.env.DISCORD_TOKEN!,
  intents: GatewayIntents.Guilds | GatewayIntents.GuildMessages
});

client.on('messageCreate', async (msg) => {
  if (msg.content === '!ping') await msg.reply('Pong! ⚡');
});

await client.login();
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
Rift v0.1.0 is live worldwide!

⭐ GitHub: https://github.com/isaim0011/rift
🌐 Website: https://rift-discord.dev/
🦀 Crates.io: https://crates.io/crates/rift-discord
📦 npm: https://www.npmjs.com/package/rift-discord
🐍 PyPI: https://pypi.org/project/rift-discord/

Drop a star on GitHub and build something fast today! ⚡
