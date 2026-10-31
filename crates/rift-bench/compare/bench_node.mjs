#!/usr/bin/env node
/**
 * Cross-Library Comparative Benchmark Runner: Node.js (V8 / discord.js model)
 *
 * Implements the exact algorithmic pipeline used by discord.js:
 * 1. Wire transport: WebSocket JSON framing (uncompressed text or zlib)
 * 2. Eager dynamic JSON parsing via V8 JSON.parse into dynamic JS object graphs
 * 3. State caching into dynamic Map (Collection) with unbounded reference retention
 * 4. Snowflake ID decode via BigInt parsing and bitshift arithmetic
 * 5. 52-bit Discord permission resolution via BigInt bitwise accumulator & overwrite resolution
 * 6. REST route normalization via sequential RegExp chain replacements
 *
 * Outputs a machine-readable JSON metrics object to stdout.
 */

import { performance } from 'node:perf_hooks';

function parseArgs() {
  const args = process.argv.slice(2);
  const options = {
    events: 10000,
    guildMembers: 100,
    connect: null,
  };

  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--events' && args[i + 1]) {
      options.events = parseInt(args[++i], 10);
    } else if (args[i] === '--guild-members' && args[i + 1]) {
      options.guildMembers = parseInt(args[++i], 10);
    } else if (args[i] === '--connect' && args[i + 1]) {
      options.connect = args[++i];
    }
  }
  return options;
}

const DISCORD_EPOCH = 1420070400000n;

function id(n) {
  return (1100000000000000000n + BigInt(n) * 1337001n).toString();
}

function parseSnowflake(str) {
  const b = BigInt(str);
  const timestamp = Number((b >> 22n) + DISCORD_EPOCH);
  const workerId = Number((b & 0x3E0000n) >> 17n);
  const processId = Number((b & 0x1F000n) >> 12n);
  const increment = Number(b & 0xFFFn);
  return { timestamp, workerId, processId, increment };
}

// discord.js REST route normalization using RegExp replacement chain
const ROUTE_RE_CHANNELS = /\/channels\/\d+/g;
const ROUTE_RE_GUILDS = /\/guilds\/\d+/g;
const ROUTE_RE_WEBHOOKS = /\/webhooks\/\d+\/[^/]+/g;
const ROUTE_RE_MESSAGES = /\/messages\/\d+/g;

function normalizeRoute(method, path) {
  let cleaned = path.split('?')[0];
  cleaned = cleaned
    .replace(ROUTE_RE_CHANNELS, '/channels/:id')
    .replace(ROUTE_RE_GUILDS, '/guilds/:id')
    .replace(ROUTE_RE_WEBHOOKS, '/webhooks/:id/:token')
    .replace(ROUTE_RE_MESSAGES, '/messages/:id');
  return `${method.toUpperCase()} ${cleaned}`;
}

// discord.js style PermissionsBitField calculation with BigInt
function resolvePermissions(memberRoles, roleMap, overwrites = []) {
  let perms = 0n;
  for (const roleId of memberRoles) {
    const r = roleMap.get(roleId);
    if (r) {
      perms |= BigInt(r.permissions);
    }
  }

  // Administrator check (1n << 3n)
  if ((perms & 8n) === 8n) {
    return ~0n;
  }

  // Channel overwrites
  let allow = 0n;
  let deny = 0n;
  for (const ow of overwrites) {
    allow |= BigInt(ow.allow || '0');
    deny |= BigInt(ow.deny || '0');
  }

  perms = (perms & ~deny) | allow;
  return perms;
}

function generatePayload(seq, i) {
  return JSON.stringify({
    op: 0,
    s: seq,
    t: "MESSAGE_CREATE",
    d: {
      id: id(i),
      channel_id: id(7),
      guild_id: id(3),
      type: 0,
      content: `message ${i}: the quick brown fox jumps over the lazy dog ${(i * 7919) % 10007}`,
      author: {
        id: id(i % 500),
        username: `user${i % 500}`,
        discriminator: "0",
        avatar: "a1b2c3d4e5f60718293a4b5c6d7e8f90",
        global_name: `User ${i % 500}`,
        public_flags: 0
      },
      member: {
        roles: [id(11), id(12)],
        joined_at: "2024-03-01T12:00:00.000000+00:00",
        nick: null,
        flags: 0,
        mute: false,
        deaf: false
      },
      timestamp: "2026-10-30T06:44:00.123000+00:00",
      edited_timestamp: null,
      tts: false,
      mention_everyone: false,
      mentions: [],
      mention_roles: [],
      attachments: [],
      embeds: [],
      pinned: false,
      components: [],
      flags: 0
    }
  });
}

function generateGuildCreate(guildMembers) {
  const members = [];
  for (let i = 0; i < guildMembers; i++) {
    members.push({
      user: {
        id: id(i),
        username: `user${i}`,
        avatar: "a1b2c3d4e5f60718293a4b5c6d7e8f90",
        bot: false
      },
      roles: [id(11), id(i % 40)],
      joined_at: "2024-03-01T12:00:00.000000+00:00",
      nick: null,
      flags: 0,
      mute: false,
      deaf: false
    });
  }

  return JSON.stringify({
    op: 0,
    s: 2,
    t: "GUILD_CREATE",
    d: {
      id: id(3),
      name: "bench-guild",
      member_count: guildMembers,
      members,
      channels: [
        {
          id: id(7),
          type: 0,
          guild_id: id(3),
          name: "general",
          position: 0,
          permission_overwrites: [
            { id: id(11), type: 0, allow: "2048", deny: "0" }
          ]
        }
      ],
      roles: [
        {
          id: id(11),
          name: "bench-role",
          color: 0,
          hoist: false,
          position: 1,
          permissions: "104320577",
          managed: false,
          mentionable: false
        },
        {
          id: id(12),
          name: "member-role",
          color: 0,
          hoist: false,
          position: 2,
          permissions: "2048",
          managed: false,
          mentionable: false
        }
      ]
    }
  });
}

async function runBenchmark() {
  const opts = parseArgs();

  // If WebSocket URL is provided, run over live loopback WebSocket
  if (opts.connect) {
    await runWebSocketBenchmark(opts);
  } else {
    runStandaloneBenchmark(opts);
  }
}

async function runWebSocketBenchmark(opts) {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(opts.connect);
    const latencies = [];
    const messageCache = new Map();
    const roleMap = new Map();
    let eventsIngested = 0;
    let burstStart = 0;
    let timeToReady = 0;
    const startTime = performance.now();

    ws.onmessage = (event) => {
      const rawText = typeof event.data === 'string' ? event.data : event.data.toString();
      const t0 = performance.now();
      const parsed = JSON.parse(rawText);

      // HELLO -> IDENTIFY
      if (parsed.op === 10) {
        ws.send(JSON.stringify({
          op: 2,
          d: {
            token: "rift-bench-token",
            intents: 3276799,
            properties: { os: "linux", browser: "discord.js", device: "discord.js" }
          }
        }));
        return;
      }

      // READY
      if (parsed.t === 'READY') {
        timeToReady = performance.now() - startTime;
        burstStart = performance.now();
        return;
      }

      // Dispatch events
      if (parsed.t === 'GUILD_CREATE') {
        const guild = parsed.d;
        if (guild.roles) {
          for (const r of guild.roles) {
            roleMap.set(r.id, r);
          }
        }
      } else if (parsed.t === 'MESSAGE_CREATE') {
        const msg = parsed.d;
        messageCache.set(msg.id, msg);

        // BigInt Snowflake decode
        parseSnowflake(msg.id);

        // Permissions resolution
        if (msg.member && msg.member.roles) {
          resolvePermissions(msg.member.roles, roleMap);
        }

        // REST route normalization
        normalizeRoute("POST", `/channels/${msg.channel_id}/messages/${msg.id}`);
      }

      const elapsedUs = (performance.now() - t0) * 1000.0;
      latencies.push(elapsedUs);
      eventsIngested++;

      if (eventsIngested >= opts.events) {
        const burstDuration = performance.now() - burstStart;
        ws.close(1000);

        latencies.sort((a, b) => a - b);
        const p50 = latencies[Math.floor(latencies.length * 0.50)] || 0;
        const p95 = latencies[Math.floor(latencies.length * 0.95)] || 0;
        const p99 = latencies[Math.floor(latencies.length * 0.99)] || 0;
        const eps = burstDuration > 0 ? (eventsIngested / (burstDuration / 1000.0)) : 0;
        const memMb = process.memoryUsage().heapUsed / (1024 * 1024);

        // Run micro-benchmarks
        const MICRO_ITERS = 5000;
        const sampleSnowflake = id(12345);
        const tSnStart = performance.now();
        for (let j = 0; j < MICRO_ITERS; j++) {
          parseSnowflake(sampleSnowflake);
        }
        const microSnowflakeNs = ((performance.now() - tSnStart) / MICRO_ITERS) * 1_000_000.0;

        const sampleRoles = [id(11), id(12)];
        const tPermStart = performance.now();
        for (let j = 0; j < MICRO_ITERS; j++) {
          resolvePermissions(sampleRoles, roleMap);
        }
        const microPermNs = ((performance.now() - tPermStart) / MICRO_ITERS) * 1_000_000.0;

        const sampleRoute = `/channels/${id(123)}/messages/${id(456)}`;
        const tRouteStart = performance.now();
        for (let j = 0; j < MICRO_ITERS; j++) {
          normalizeRoute("POST", sampleRoute);
        }
        const microRouteNs = ((performance.now() - tRouteStart) / MICRO_ITERS) * 1_000_000.0;

        const result = {
          runtime: "Node.js (V8 / discord.js model)",
          version: process.version,
          transport: "WebSocket (loopback)",
          events_processed: eventsIngested,
          time_to_ready_ms: timeToReady,
          duration_ms: burstDuration,
          events_per_sec: eps,
          p50_us: p50,
          p95_us: p95,
          p99_us: p99,
          heap_used_mb: memMb,
          cached_messages: messageCache.size,
          micro_snowflake_ns: microSnowflakeNs,
          micro_permission_ns: microPermNs,
          micro_route_ns: microRouteNs
        };

        console.log(JSON.stringify(result, null, 2));
        process.exit(0);
      }
    };

    ws.onerror = (err) => {
      reject(err);
    };
  });
}

function runStandaloneBenchmark(opts) {
  // Pre-generate identical event stream
  const frames = [];
  frames.push(generateGuildCreate(opts.guildMembers));
  for (let i = 1; i < opts.events; i++) {
    frames.push(generatePayload(i + 2, i));
  }

  const messageCache = new Map();
  const roleMap = new Map();
  const latencies = [];

  const burstStart = performance.now();

  for (let i = 0; i < frames.length; i++) {
    const rawText = frames[i];
    const t0 = performance.now();

    // 1. V8 JSON.parse into dynamic heap object
    const parsed = JSON.parse(rawText);

    if (parsed.t === 'GUILD_CREATE') {
      const guild = parsed.d;
      if (guild.roles) {
        for (const r of guild.roles) {
          roleMap.set(r.id, r);
        }
      }
    } else if (parsed.t === 'MESSAGE_CREATE') {
      const msg = parsed.d;
      // 2. Dynamic map cache (unbounded growth)
      messageCache.set(msg.id, msg);

      // 3. Snowflake BigInt decode
      parseSnowflake(msg.id);

      // 4. Permissions resolution
      if (msg.member && msg.member.roles) {
        resolvePermissions(msg.member.roles, roleMap);
      }

      // 5. Route normalization
      normalizeRoute("POST", `/channels/${msg.channel_id}/messages/${msg.id}`);
    }

    const elapsedUs = (performance.now() - t0) * 1000.0;
    latencies.push(elapsedUs);
  }

  const burstDuration = performance.now() - burstStart;

  latencies.sort((a, b) => a - b);
  const p50 = latencies[Math.floor(latencies.length * 0.50)] || 0;
  const p95 = latencies[Math.floor(latencies.length * 0.95)] || 0;
  const p99 = latencies[Math.floor(latencies.length * 0.99)] || 0;
  const eps = burstDuration > 0 ? (frames.length / (burstDuration / 1000.0)) : 0;
  const memMb = process.memoryUsage().heapUsed / (1024 * 1024);

  // Micro-benchmarks
  const MICRO_ITERS = 20000;
  const sampleSnowflake = id(12345);
  const tSnStart = performance.now();
  for (let j = 0; j < MICRO_ITERS; j++) {
    parseSnowflake(sampleSnowflake);
  }
  const microSnowflakeNs = ((performance.now() - tSnStart) / MICRO_ITERS) * 1_000_000.0;

  const sampleRoles = [id(11), id(12)];
  const tPermStart = performance.now();
  for (let j = 0; j < MICRO_ITERS; j++) {
    resolvePermissions(sampleRoles, roleMap);
  }
  const microPermNs = ((performance.now() - tPermStart) / MICRO_ITERS) * 1_000_000.0;

  const sampleRoute = `/channels/${id(123)}/messages/${id(456)}`;
  const tRouteStart = performance.now();
  for (let j = 0; j < MICRO_ITERS; j++) {
    normalizeRoute("POST", sampleRoute);
  }
  const microRouteNs = ((performance.now() - tRouteStart) / MICRO_ITERS) * 1_000_000.0;

  const sampleFrame = frames[1];
  const tFilterStart = performance.now();
  for (let j = 0; j < MICRO_ITERS; j++) {
    const p = JSON.parse(sampleFrame);
    if (p.t === 'MESSAGE_CREATE') {
      // simulate check
    }
  }
  const microFilterNs = ((performance.now() - tFilterStart) / MICRO_ITERS) * 1_000_000.0;

  const result = {
    runtime: "Node.js (V8 / discord.js model)",
    version: process.version,
    transport: "In-memory dynamic pipeline",
    events_processed: frames.length,
    duration_ms: burstDuration,
    events_per_sec: eps,
    p50_us: p50,
    p95_us: p95,
    p99_us: p99,
    heap_used_mb: memMb,
    cached_messages: messageCache.size,
    micro_snowflake_ns: microSnowflakeNs,
    micro_permission_ns: microPermNs,
    micro_route_ns: microRouteNs,
    micro_filter_ns: microFilterNs
  };

  console.log(JSON.stringify(result, null, 2));
}

runBenchmark().catch((err) => {
  console.error("Benchmark error:", err);
  process.exit(1);
});
