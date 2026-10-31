"use strict";

const fs = require("fs");
const path = require("path");

// Load native binary if compiled
let nativeBinding = null;
const candidates = [
  path.join(__dirname, "rift-node.win32-x64-msvc.node"),
  path.join(__dirname, "rift-node.linux-x64-gnu.node"),
  path.join(__dirname, "rift-node.darwin-arm64.node"),
  path.join(__dirname, "rift-node.darwin-x64.node"),
  path.join(__dirname, "rift-node.node"),
  path.join(__dirname, "../../target/release/rift_node.node"),
  path.join(__dirname, "../../target/debug/rift_node.node"),
];

for (const candidate of candidates) {
  if (fs.existsSync(candidate)) {
    try {
      nativeBinding = require(candidate);
      break;
    } catch (_) {}
  }
}

// Universal bridge dispatch helper
function dispatch(method, params) {
  if (nativeBinding && nativeBinding.dispatch_bridge) {
    const raw = nativeBinding.dispatch_bridge(method, JSON.stringify(params));
    return JSON.parse(raw);
  }
  return null;
}

// ----------------------------------------------------
// 1. Snowflake
// ----------------------------------------------------
const DISCORD_EPOCH_MS = 1420070400000n;

const Snowflake = {
  timestampMs(id) {
    const n = BigInt(id);
    return Number((n >> 22n) + DISCORD_EPOCH_MS);
  },
  workerId(id) {
    const n = BigInt(id);
    return Number((n & 0x3e0000n) >> 17n);
  },
  processId(id) {
    const n = BigInt(id);
    return Number((n & 0x1f000n) >> 12n);
  },
  increment(id) {
    const n = BigInt(id);
    return Number(n & 0xfffn);
  },
  fromParts(timestampMs, workerId, processId, increment) {
    const tsOffset = BigInt(timestampMs) - DISCORD_EPOCH_MS;
    const sf =
      (tsOffset << 22n) |
      ((BigInt(workerId) & 0x1fn) << 17n) |
      ((BigInt(processId) & 0x1fn) << 12n) |
      (BigInt(increment) & 0xfffn);
    return sf.toString();
  },
  deconstruct(id) {
    return {
      timestampMs: this.timestampMs(id),
      workerId: this.workerId(id),
      processId: this.processId(id),
      increment: this.increment(id),
    };
  },
};

// ----------------------------------------------------
// 2. Permissions
// ----------------------------------------------------
const Permissions = {
  CREATE_INSTANT_INVITE: 1n << 0n,
  KICK_MEMBERS: 1n << 1n,
  BAN_MEMBERS: 1n << 2n,
  ADMINISTRATOR: 1n << 3n,
  MANAGE_CHANNELS: 1n << 4n,
  MANAGE_GUILD: 1n << 5n,
  ADD_REACTIONS: 1n << 6n,
  VIEW_AUDIT_LOG: 1n << 7n,
  PRIORITY_SPEAKER: 1n << 8n,
  STREAM: 1n << 9n,
  VIEW_CHANNEL: 1n << 10n,
  SEND_MESSAGES: 1n << 11n,
  SEND_TTS_MESSAGES: 1n << 12n,
  MANAGE_MESSAGES: 1n << 13n,
  EMBED_LINKS: 1n << 14n,
  ATTACH_FILES: 1n << 15n,
  READ_MESSAGE_HISTORY: 1n << 16n,
  MENTION_EVERYONE: 1n << 17n,
  USE_EXTERNAL_EMOJIS: 1n << 18n,
  VIEW_GUILD_INSIGHTS: 1n << 19n,
  CONNECT: 1n << 20n,
  SPEAK: 1n << 21n,
  MUTE_MEMBERS: 1n << 22n,
  DEAFEN_MEMBERS: 1n << 23n,
  MOVE_MEMBERS: 1n << 24n,
  USE_VAD: 1n << 25n,
  CHANGE_NICKNAME: 1n << 26n,
  MANAGE_NICKNAMES: 1n << 27n,
  MANAGE_ROLES: 1n << 28n,
  MANAGE_WEBHOOKS: 1n << 29n,
  MANAGE_GUILD_EXPRESSIONS: 1n << 30n,
  USE_APPLICATION_COMMANDS: 1n << 31n,
  REQUEST_TO_SPEAK: 1n << 32n,
  MANAGE_EVENTS: 1n << 33n,
  MANAGE_THREADS: 1n << 34n,
  CREATE_PUBLIC_THREADS: 1n << 35n,
  CREATE_PRIVATE_THREADS: 1n << 36n,
  USE_EXTERNAL_STICKERS: 1n << 37n,
  SEND_MESSAGES_IN_THREADS: 1n << 38n,
  USE_EMBEDDED_ACTIVITIES: 1n << 39n,
  MODERATE_MEMBERS: 1n << 40n,
  VIEW_CREATOR_MONETIZATION_ANALYTICS: 1n << 41n,
  USE_SOUNDBOARD: 1n << 42n,
  CREATE_GUILD_EXPRESSIONS: 1n << 43n,
  CREATE_EVENTS: 1n << 44n,
  USE_EXTERNAL_SOUNDS: 1n << 45n,
  SEND_VOICE_MESSAGES: 1n << 46n,
  SET_VOICE_CHANNEL_STATUS: 1n << 48n,
  SEND_POLLS: 1n << 49n,
  USE_EXTERNAL_APPS: 1n << 50n,
  PIN_MESSAGES: 1n << 51n,
  BYPASS_SLOWMODE: 1n << 52n,

  hasPermission(permissions, required) {
    const p = BigInt(permissions);
    const r = BigInt(required);
    if ((p & Permissions.ADMINISTRATOR) === Permissions.ADMINISTRATOR) {
      return true;
    }
    return (p & r) === r;
  },

  computeChannelPermissions(basePermissions, overwrites, guildId, memberId, memberRoles) {
    let p = BigInt(basePermissions);
    if ((p & Permissions.ADMINISTRATOR) === Permissions.ADMINISTRATOR) {
      return Permissions.ALL.toString();
    }

    const gId = String(guildId);
    const mId = String(memberId);
    const rIds = new Set((memberRoles || []).map(String));

    // 1. @everyone overwrite
    const everyoneOw = (overwrites || []).find((o) => o.type === 0 && String(o.id) === gId);
    if (everyoneOw) {
      p = (p & ~BigInt(everyoneOw.deny)) | BigInt(everyoneOw.allow);
    }

    // 2. Role overwrites (allow beats deny across roles)
    let roleAllow = 0n;
    let roleDeny = 0n;
    for (const ow of overwrites || []) {
      if (ow.type === 0 && rIds.has(String(ow.id))) {
        roleAllow |= BigInt(ow.allow);
        roleDeny |= BigInt(ow.deny);
      }
    }
    p = (p & ~roleDeny) | roleAllow;

    // 3. Member overwrite
    const memberOw = (overwrites || []).find((o) => o.type === 1 && String(o.id) === mId);
    if (memberOw) {
      p = (p & ~BigInt(memberOw.deny)) | BigInt(memberOw.allow);
    }

    // 4. Implicit text rules
    if ((p & Permissions.VIEW_CHANNEL) === 0n) {
      p = 0n;
    } else if ((p & Permissions.SEND_MESSAGES) === 0n) {
      const dependent =
        Permissions.MENTION_EVERYONE |
        Permissions.SEND_TTS_MESSAGES |
        Permissions.ATTACH_FILES |
        Permissions.EMBED_LINKS;
      p &= ~dependent;
    }

    return p.toString();
  },
};

Permissions.ALL =
  (1n << 53n) - 1n - (1n << 47n); // bits 0..=46 and 48..=52

// ----------------------------------------------------
// 3. Intents & Capabilities
// ----------------------------------------------------
const Intents = {
  GUILDS: 1 << 0,
  GUILD_MEMBERS: 1 << 1,
  GUILD_MODERATION: 1 << 2,
  GUILD_EXPRESSIONS: 1 << 3,
  GUILD_INTEGRATIONS: 1 << 4,
  GUILD_WEBHOOKS: 1 << 5,
  GUILD_INVITES: 1 << 6,
  GUILD_VOICE_STATES: 1 << 7,
  GUILD_PRESENCES: 1 << 8,
  GUILD_MESSAGES: 1 << 9,
  GUILD_MESSAGE_REACTIONS: 1 << 10,
  GUILD_MESSAGE_TYPING: 1 << 11,
  DIRECT_MESSAGES: 1 << 12,
  DIRECT_MESSAGE_REACTIONS: 1 << 13,
  DIRECT_MESSAGE_TYPING: 1 << 14,
  MESSAGE_CONTENT: 1 << 15,
  GUILD_SCHEDULED_EVENTS: 1 << 16,
  AUTO_MODERATION_CONFIGURATION: 1 << 20,
  AUTO_MODERATION_EXECUTION: 1 << 21,
  GUILD_MESSAGE_POLLS: 1 << 24,
  DIRECT_MESSAGE_POLLS: 1 << 25,
  PRIVILEGED: (1 << 1) | (1 << 8) | (1 << 15),
  ALL_KNOWN:
    (1 << 0) |
    (1 << 1) |
    (1 << 2) |
    (1 << 3) |
    (1 << 4) |
    (1 << 5) |
    (1 << 6) |
    (1 << 7) |
    (1 << 8) |
    (1 << 9) |
    (1 << 10) |
    (1 << 11) |
    (1 << 12) |
    (1 << 13) |
    (1 << 14) |
    (1 << 15) |
    (1 << 16) |
    (1 << 20) |
    (1 << 21) |
    (1 << 24) |
    (1 << 25),
};

const Capabilities = {
  NONE: 0,
  CHANNEL_OBFUSCATION: 1 << 15,
  IS_COMPONENTS_V2: 1 << 16,
};

// ----------------------------------------------------
// 4. verifyInteraction
// ----------------------------------------------------
function verifyInteraction(publicKeyHex, signatureHex, timestamp, body) {
  if (nativeBinding && nativeBinding.verify_interaction) {
    const buf = Buffer.isBuffer(body) ? body : Buffer.from(body);
    return nativeBinding.verify_interaction(publicKeyHex, signatureHex, timestamp, buf);
  }

  // Fallback: use node:crypto Ed25519
  try {
    const crypto = require("crypto");
    const pubKeyBuf = Buffer.from(publicKeyHex, "hex");
    if (pubKeyBuf.length !== 32) return false;
    const sigBuf = Buffer.from(signatureHex, "hex");
    if (sigBuf.length !== 64) return false;

    // Convert raw public key to DER/SPKI format
    const prefix = Buffer.from("302a300506032b6570032100", "hex");
    const spki = Buffer.concat([prefix, pubKeyBuf]);
    const key = crypto.createPublicKey({ key: spki, format: "der", type: "spki" });

    const bodyBuf = Buffer.isBuffer(body) ? body : Buffer.from(body);
    const data = Buffer.concat([Buffer.from(timestamp), bodyBuf]);

    return crypto.verify(null, data, key, sigBuf);
  } catch (_) {
    return false;
  }
}

// ----------------------------------------------------
// 5. RateLimiter & normalizeRoute
// ----------------------------------------------------
function normalizeRoute(method, pathStr) {
  const p = (pathStr || "").split("?")[0];
  const segs = p.split("/").filter(Boolean);

  if (segs[0] === "interactions") {
    const rest = segs.slice(3).map((s) => "/" + s).join("");
    const template = `/interactions/:id/:token${rest}`;
    return {
      id: `${method.toUpperCase()} ${template}`,
      template,
      major: "interactions",
    };
  }

  const out = [];
  let major = "";
  let i = 0;
  while (i < segs.len || i < segs.length) {
    const s = segs[i];
    const prev = i > 0 ? segs[i - 1] : "";
    const isSnowflake = /^\d+$/.test(s);

    if (isSnowflake && (prev === "channels" || prev === "guilds")) {
      if (!major) major = `${prev}/${s}`;
      out.push(":major");
    } else if (isSnowflake && prev === "webhooks") {
      let m = `webhooks/${s}`;
      if (segs[i + 1]) {
        m += `/${segs[i + 1]}`;
        i++;
      }
      if (!major) major = m;
      out.push(":major");
    } else if (isSnowflake) {
      out.push(":id");
    } else if (prev === "reactions") {
      out.push(":emoji");
    } else if (prev === "invites") {
      out.push(":code");
    } else {
      out.push(s);
    }
    i++;
  }

  const template = `/${out.join("/")}`;
  return {
    id: `${method.toUpperCase()} ${template}`,
    template,
    major,
  };
}

class RateLimiter {
  constructor(globalPerSecond = 50, invalidLimit = 10000, invalidTripAt = 9000) {
    this.globalPerSecond = globalPerSecond;
    this.invalidLimit = invalidLimit;
    this.invalidTripAt = invalidTripAt;
    this.buckets = new Map();
    this.routeBucket = new Map();
    this.invalidLog = [];
  }

  acquire(method, path, nowMs = Date.now()) {
    const norm = normalizeRoute(method, path);
    const key = `${norm.template}|${norm.major}`;
    const b = this.buckets.get(key);

    if (!b) {
      // First probe
      this.buckets.set(key, { limit: 1, remaining: 0, resetAt: nowMs + 30000, probing: true });
      return { verdict: "go" };
    }

    if (b.probing) {
      return { verdict: "wait", retryAfterMs: 50 };
    }

    if (nowMs >= b.resetAt) {
      b.remaining = b.limit;
    }

    if (b.remaining > 0) {
      b.remaining--;
      return { verdict: "go" };
    }

    const wait = Math.max(0, b.resetAt - nowMs);
    return { verdict: "wait", retryAfterMs: wait };
  }

  onResponse(method, path, status, headers = {}, nowMs = Date.now()) {
    const norm = normalizeRoute(method, path);
    const key = `${norm.template}|${norm.major}`;

    const limit = headers.limit !== undefined ? headers.limit : 1;
    const remaining = headers.remaining !== undefined ? headers.remaining : 1;
    const resetAfter = headers.resetAfterMs !== undefined ? headers.resetAfterMs : 1000;

    this.buckets.set(key, {
      limit,
      remaining,
      resetAt: nowMs + resetAfter,
      probing: false,
    });

    if (status === 401 || status === 403 || status === 429) {
      if (headers.scope !== "shared") {
        this.invalidLog.push(nowMs);
      }
    }
  }

  invalidRequests(nowMs = Date.now()) {
    const tenMinAgo = nowMs - 600000;
    this.invalidLog = this.invalidLog.filter((t) => t >= tenMinAgo);
    return this.invalidLog.length;
  }
}

// ----------------------------------------------------
// 6. Cache
// ----------------------------------------------------
class Cache {
  constructor(config = {}) {
    this.guilds = new Map();
    this.channels = new Map();
    this.obfuscated = new Set();
    this.members = new Map();
    this.roles = new Map();
    this.messages = new Map();
    this.users = new Map();
  }

  update(eventOrEnvelope) {
    const evt = typeof eventOrEnvelope === "string" ? JSON.parse(eventOrEnvelope) : eventOrEnvelope;
    const t = evt.t || evt.name;
    const d = evt.d || evt.data || evt;

    if (t === "GUILD_CREATE") {
      this.guilds.set(String(d.id), d);
      if (Array.isArray(d.channels)) {
        for (const ch of d.channels) {
          this._insertChannel(ch);
        }
      }
    } else if (t === "CHANNEL_CREATE" || t === "CHANNEL_UPDATE") {
      this._insertChannel(d);
    } else if (t === "CHANNEL_DELETE") {
      this.channels.delete(String(d.id));
      this.obfuscated.delete(String(d.id));
    }
  }

  _insertChannel(ch) {
    const id = String(ch.id);
    const isObfuscated = (ch.flags & (1 << 17)) !== 0;
    if (isObfuscated) {
      this.obfuscated.add(id);
      this.channels.set(id, { ...ch, name: null, topic: null, permission_overwrites: [] });
    } else {
      this.obfuscated.delete(id);
      this.channels.set(id, ch);
    }
  }

  isChannelObfuscated(channelId) {
    return this.obfuscated.has(String(channelId));
  }

  getGuild(guildId) {
    return this.guilds.get(String(guildId)) || null;
  }

  getChannel(channelId) {
    if (this.isChannelObfuscated(channelId)) return null;
    return this.channels.get(String(channelId)) || null;
  }

  getMember(guildId, userId) {
    const key = `${guildId}/${userId}`;
    return this.members.get(key) || null;
  }

  getMessage(channelId, messageId) {
    const key = `${channelId}/${messageId}`;
    return this.messages.get(key) || null;
  }

  computeMemberPermissions(guildId, channelId, userId) {
    if (this.isChannelObfuscated(channelId)) return "0";
    const ch = this.channels.get(String(channelId));
    if (!ch) return "0";
    return Permissions.computeChannelPermissions(
      Permissions.ALL,
      ch.permission_overwrites || [],
      guildId,
      userId,
      []
    );
  }

  stats() {
    return {
      guilds: this.guilds.size,
      visibleChannels: Math.max(0, this.channels.size - this.obfuscated.size),
      obfuscatedChannels: this.obfuscated.size,
      roles: this.roles.size,
      members: this.members.size,
      users: this.users.size,
      messages: this.messages.size,
    };
  }

  clear() {
    this.guilds.clear();
    this.channels.clear();
    this.obfuscated.clear();
    this.members.clear();
    this.roles.clear();
    this.messages.clear();
    this.users.clear();
  }
}

// ----------------------------------------------------
// 7. GatewaySession
// ----------------------------------------------------
class GatewaySession {
  constructor() {
    this.reset();
  }

  reset() {
    this.intervalMs = 0;
    this.nextDueMs = 0;
    this.awaitingAck = false;
    this.started = false;
    this.sequence = null;
    this.sessionId = null;
    this.resumeUrl = null;
  }

  onHello(nowMs, intervalMs, jitter = 0.5) {
    const j = Math.min(1.0, Math.max(0.0, jitter));
    this.intervalMs = intervalMs;
    this.nextDueMs = nowMs + Math.floor(intervalMs * j);
    this.awaitingAck = false;
    this.started = true;
  }

  onReady(sessionId, resumeGatewayUrl) {
    this.sessionId = sessionId;
    this.resumeUrl = resumeGatewayUrl;
  }

  recordSeq(seq) {
    this.sequence = seq;
  }

  seq() {
    return this.sequence;
  }

  isEstablished() {
    return this.sessionId !== null;
  }

  pollHeartbeat(nowMs) {
    if (!this.started || nowMs < this.nextDueMs) {
      return { action: "wait" };
    }
    if (this.awaitingAck) {
      return { action: "zombie" };
    }
    this.awaitingAck = true;
    this.nextDueMs = nowMs + this.intervalMs;
    return { action: "send", seq: this.sequence };
  }

  planReconnect(why, closeCode, resumable = false) {
    if (closeCode === 4004 || closeCode === 4010 || closeCode === 4011 || closeCode === 4012 || closeCode === 4013 || closeCode === 4014) {
      return { plan: "fatal", closeCode };
    }

    if (why === "invalid_session" && !resumable) {
      this.reset();
      return { plan: "identify" };
    }

    if (this.sessionId && this.resumeUrl && this.sequence !== null) {
      return {
        plan: "resume",
        url: this.resumeUrl,
        sessionId: this.sessionId,
        seq: this.sequence,
      };
    }

    this.reset();
    return { plan: "identify" };
  }
}

// ----------------------------------------------------
// 8. RiftClient
// ----------------------------------------------------
class RiftClient {
  constructor(token, intents = Intents.ALL_KNOWN, capabilities = Capabilities.NONE) {
    this.token = token;
    this.intents = intents;
    this.capabilities = capabilities;
    this.eventQueue = [];
    this.resolvers = [];
    this.stopped = false;
  }

  injectMockEvent(shardId, name, payload) {
    const evt = { shardId, name, data: payload };
    if (this.resolvers.length > 0) {
      const resolve = this.resolvers.shift();
      resolve(evt);
    } else {
      this.eventQueue.push(evt);
    }
  }

  async nextEvent() {
    if (this.stopped) return null;
    if (this.eventQueue.length > 0) {
      return this.eventQueue.shift();
    }
    return new Promise((resolve) => {
      this.resolvers.push(resolve);
    });
  }

  sendCommand(shardId, opcode, payload) {
    // Queued or handled in custom transport
  }

  updatePresence(shardId, status, activityName, activityType = 0) {
    const payload = {
      status,
      since: null,
      afk: false,
      activities: activityName ? [{ name: activityName, type: activityType }] : [],
    };
    this.sendCommand(shardId, 3, payload);
  }

  async stop() {
    this.stopped = true;
    while (this.resolvers.length > 0) {
      const resolve = this.resolvers.shift();
      resolve(null);
    }
  }
}

module.exports = {
  Snowflake,
  Permissions,
  Intents,
  Capabilities,
  verifyInteraction,
  normalizeRoute,
  RateLimiter,
  Cache,
  GatewaySession,
  RiftClient,
};
