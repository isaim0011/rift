# Show HN: Rift – Zero-unsafe, sans-I/O Discord engine in Rust with Node/Bun/Python bindings

**URL**: https://github.com/isaim0011/rift (or https://rift-discord.dev/)

**Title**: Show HN: Rift – Zero-unsafe, sans-I/O Discord engine with Node, Bun, and Python bindings

### Submission Text:
Hey HN,

I built Rift (https://rift-discord.dev/), a high-performance Discord v10 engine with three goals that existing libraries rarely satisfy at once:

1. **Pure Sans-I/O Architecture**: All Discord protocol logic (gateway state machines, sequence reconciliation, shard orchestration, heartbeat management, and rate limit token buckets) is completely decoupled from network transports. You can test complete multi-shard failover scenarios deterministically in unit tests without spinning up mock servers or network listeners.

2. **Strict Zero-Unsafe Guarantee**: Every single crate in the workspace compiles under `#![forbid(unsafe_code)]`. Memory safety is enforced by the compiler without compromise.

3. **True Multi-Language Polyglot Bindings**: We wanted Python, Node.js, and Bun developers to benefit from Rust's performance (measured at 96,981 events/sec full stack and 171,226 events/sec raw on loopback) without having to write Rust. We exposed clean, idiomatic bindings using napi-rs and PyO3 so you can write in TypeScript:

```typescript
import { RiftClient, Intents } from 'rift-discord';
const client = new RiftClient(process.env.DISCORD_TOKEN!, Intents.GUILDS | Intents.GUILD_MESSAGES);
while (true) {
  const event = await client.nextEvent();
  if (!event) break;
  if (event.name === 'MESSAGE_CREATE' && event.data?.content === '!ping') {
    console.log(`Pong from ${event.data.author?.username}! ⚡`);
  }
}
```

And in Python:
```python
import asyncio
from rift import RiftClient, Intents
async def main():
    client = RiftClient(os.environ["DISCORD_TOKEN"], intents=Intents.GUILDS | Intents.GUILD_MESSAGES)
    while True:
        event = await client.next_event()
        if event is None: break
        if event.name == "MESSAGE_CREATE" and event.data.get("content") == "!ping":
            print(f"Pong from {event.data['author']['username']}! ⚡")
asyncio.run(main())
```

### Why Another Discord Library?
Existing JS and Python solutions struggle under high event load, often accumulating garbage collection pressure and dropping heartbeat intervals during event surges. Rift combines the performance of a zero-unsafe Rust engine with the ergonomics of modern scripting runtimes.

We also built an official Model Context Protocol (MCP) server ready to deploy serverless on Cloudflare Workers so AI agents (Claude, Cursor, Antigravity) can generate and audit type-safe Discord bots with live OpenAPI route validation.

- **GitHub**: https://github.com/isaim0011/rift
- **Crates.io**: `cargo add rift-discord`
- **npm / Bun**: `npm install rift-discord` / `bun add rift-discord`
- **PyPI**: `pip install rift-discord`
- **Coverage**: 100% of Discord REST routes (250/250) verified against OpenAPI 3.1.

I’d love your feedback on the architecture and API design!
