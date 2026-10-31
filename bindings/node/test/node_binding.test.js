const test = require("node:test");
const assert = require("node:assert/strict");
const {
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
} = require("../index.js");

test("Snowflake decoding and reconstruction", () => {
  const example = "175928847299117063";
  assert.equal(Snowflake.timestampMs(example), 1462015105796);
  assert.equal(Snowflake.workerId(example), 1);
  assert.equal(Snowflake.processId(example), 0);
  assert.equal(Snowflake.increment(example), 7);

  const parts = Snowflake.deconstruct(example);
  assert.deepEqual(parts, {
    timestampMs: 1462015105796,
    workerId: 1,
    processId: 0,
    increment: 7,
  });

  const reconstructed = Snowflake.fromParts(1462015105796, 1, 0, 7);
  assert.equal(reconstructed, example);
});

test("Permissions constants and evaluation", () => {
  assert.equal(
    Permissions.hasPermission(
      Permissions.ADMINISTRATOR,
      Permissions.SEND_MESSAGES
    ),
    true
  );
  assert.equal(
    Permissions.hasPermission(
      Permissions.SEND_MESSAGES | Permissions.VIEW_CHANNEL,
      Permissions.SEND_MESSAGES
    ),
    true
  );
  assert.equal(
    Permissions.hasPermission(Permissions.VIEW_CHANNEL, Permissions.SEND_MESSAGES),
    false
  );
});

test("Permissions channel overwrite calculation", () => {
  const guildId = "1000";
  const memberId = "2000";
  const roleA = "3001";
  const roleB = "3002";

  const overwrites = [
    { id: roleA, type: 0, allow: 0n, deny: Permissions.SEND_MESSAGES },
    { id: roleB, type: 0, allow: Permissions.SEND_MESSAGES, deny: 0n },
  ];

  const computed = Permissions.computeChannelPermissions(
    Permissions.VIEW_CHANNEL,
    overwrites,
    guildId,
    memberId,
    [roleA, roleB]
  );

  const p = BigInt(computed);
  assert.notEqual(p & Permissions.SEND_MESSAGES, 0n);
});

test("Intents and Capabilities constants", () => {
  assert.equal(Intents.GUILDS, 1 << 0);
  assert.equal(Intents.MESSAGE_CONTENT, 1 << 15);
  assert.equal(Capabilities.CHANNEL_OBFUSCATION, 1 << 15);
  assert.equal(Capabilities.IS_COMPONENTS_V2, 1 << 16);
});

test("verifyInteraction signature verification handling", () => {
  const pk = "a".repeat(64);
  const sig = "b".repeat(128);
  const ts = "1700000000";
  const body = "{\"type\":1}";
  // Malformed signature should safely return false, never throw
  assert.equal(verifyInteraction(pk, sig, ts, body), false);
});

test("normalizeRoute and RateLimiter", () => {
  const route = normalizeRoute("GET", "/channels/12345/messages/67890?limit=25");
  assert.equal(route.template, "/channels/:major/messages/:id");
  assert.equal(route.major, "channels/12345");

  const limiter = new RateLimiter();
  const v1 = limiter.acquire("GET", "/channels/12345/messages");
  assert.equal(v1.verdict, "go");

  const v2 = limiter.acquire("GET", "/channels/12345/messages");
  assert.equal(v2.verdict, "wait");

  limiter.onResponse("GET", "/channels/12345/messages", 200, {
    limit: 5,
    remaining: 4,
    resetAfterMs: 1000,
  });

  const v3 = limiter.acquire("GET", "/channels/12345/messages");
  assert.equal(v3.verdict, "go");
});

test("Cache and Nov 16 Obfuscation safety", () => {
  const cache = new Cache();
  assert.equal(cache.stats().guilds, 0);

  // Normal channel
  cache.update({
    t: "CHANNEL_CREATE",
    d: { id: "101", type: 0, name: "general", flags: 0 },
  });
  assert.equal(cache.isChannelObfuscated("101"), false);
  assert.notEqual(cache.getChannel("101"), null);

  // Obfuscated channel (bit 17 = 131072)
  cache.update({
    t: "CHANNEL_CREATE",
    d: { id: "102", type: 0, name: "___hidden___", flags: 131072 },
  });
  assert.equal(cache.isChannelObfuscated("102"), true);
  assert.equal(cache.getChannel("102"), null);
  assert.equal(cache.computeMemberPermissions("1000", "102", "2000"), "0");
});

test("GatewaySession state machine and reconnection", () => {
  const session = new GatewaySession();
  assert.equal(session.isEstablished(), false);

  session.onHello(1000, 41250, 0.5);
  session.onReady("sess_abc", "wss://resume.discord.gg");
  session.recordSeq(100);
  assert.equal(session.isEstablished(), true);
  assert.equal(session.seq(), 100);

  const poll = session.pollHeartbeat(1000);
  assert.equal(poll.action, "wait");

  // Reconnect plan
  const plan = session.planReconnect("close", 4000);
  assert.equal(plan.plan, "resume");
  assert.equal(plan.url, "wss://resume.discord.gg");
  assert.equal(plan.sessionId, "sess_abc");
  assert.equal(plan.seq, 100);

  // Fatal plan
  const fatalPlan = session.planReconnect("close", 4004);
  assert.equal(fatalPlan.plan, "fatal");
  assert.equal(fatalPlan.closeCode, 4004);
});

test("RiftClient asynchronous events and presence", async () => {
  const client = new RiftClient("FAKE_TOKEN");
  client.injectMockEvent(0, "MESSAGE_CREATE", { content: "hello world" });

  const event = await client.nextEvent();
  assert.equal(event.shardId, 0);
  assert.equal(event.name, "MESSAGE_CREATE");
  assert.equal(event.data.content, "hello world");

  client.updatePresence(0, "online", "Testing Bot", 0);
  await client.stop();
  const next = await client.nextEvent();
  assert.equal(next, null);
});
