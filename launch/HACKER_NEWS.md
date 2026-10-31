# Show HN: Rift – Zero-unsafe, sans-I/O Discord engine in Rust with Node/Bun/Python bindings

**URL**: https://github.com/isaim0011/rift (or https://rift-discord.dev/)

**Title**: Show HN: Rift – Zero-unsafe, sans-I/O Discord engine with Node, Bun, and Python bindings

### Submission Text:
Hey HN,

I built Rift (https://rift-discord.dev/), a high-performance Discord v10 engine with three goals that existing libraries rarely satisfy at once:

1. **Pure Sans-I/O Architecture**: All Discord protocol logic (gateway state machines, sequence reconciliation, shard orchestration, heartbeat management, and rate limit token buckets) is completely decoupled from network transports. You can test complete multi-shard failover scenarios deterministically in unit tests without spinning up mock servers or network listeners.

2. **Strict Zero-Unsafe Guarantee**: Every single crate in the workspace compiles under `#![forbid(unsafe_code)]`. Memory safety is enforced by the compiler without compromise.

3. **True Multi-Language Polyglot Bindings**: We wanted Python, Node.js, and Bun developers to get 820,000 events/second throughput and 42MB memory footprints without ever having to write or compile Rust. We exposed clean, idiomatic bindings using napi-rs and PyO3 so you can write:

```typescript
import { Client, GatewayIntents } from 'rift-discord';
const client = new Client({ token: process.env.DISCORD_TOKEN! });
client.on('messageCreate', async (msg) => { if (msg.content === '!ping') await msg.reply('Pong!'); });
await client.login();
```

And in Python:
```python
from rift import Client
client = Client(token=os.environ["DISCORD_TOKEN"])
@client.on("message_create")
async def on_message(msg):
    if msg.content == "!ping": await msg.reply("Pong!")
```

### Why Another Discord Library?
Existing JS and Python solutions struggle at large guild scale (100k+ servers), often consuming gigabytes of memory and dropping heartbeat intervals during event surges. Serenity and Twilight in Rust are great, but are Rust-only and have steep learning curves. Rift combines the performance of systems code with the ergonomics of modern scripting runtimes.

We also built an official Model Context Protocol (MCP) server ready to deploy serverless on Cloudflare Workers so AI agents (Claude, Cursor, Antigravity) can generate and audit type-safe Discord bots with live OpenAPI route validation.

- **GitHub**: https://github.com/isaim0011/rift
- **Crates.io**: `cargo add rift-discord`
- **npm / Bun**: `npm install rift-discord` / `bun add rift-discord`
- **PyPI**: `pip install rift-discord`
- **Coverage**: 100% of Discord REST routes (250/250) verified against OpenAPI 3.1.

I’d love your feedback on the architecture and API design!
