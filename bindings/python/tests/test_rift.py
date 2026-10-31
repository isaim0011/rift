import asyncio
import pytest
import sys
import os

# Add python source to sys.path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "python")))

from rift import (
    Snowflake,
    Permissions,
    Intents,
    Capabilities,
    verify_interaction,
    normalize_route,
    RateLimiter,
    Cache,
    GatewaySession,
    RiftClient,
)


def test_snowflake_decoding_and_generation():
    example = 175928847299117063
    assert Snowflake.timestamp_ms(example) == 1462015105796
    assert Snowflake.worker_id(example) == 1
    assert Snowflake.process_id(example) == 0
    assert Snowflake.increment(example) == 7

    parts = Snowflake.deconstruct(example)
    assert parts == (1462015105796, 1, 0, 7)

    reconstructed = Snowflake.from_parts(1462015105796, 1, 0, 7)
    assert reconstructed == example


def test_permissions_evaluation_and_channels():
    assert Permissions.has_permission(Permissions.ADMINISTRATOR, Permissions.SEND_MESSAGES) is True
    assert Permissions.has_permission(Permissions.VIEW_CHANNEL, Permissions.SEND_MESSAGES) is False

    guild_id = 1000
    member_id = 2000
    role_a = 3001
    role_b = 3002

    overwrites = [
        (role_a, 0, 0, Permissions.SEND_MESSAGES),
        (role_b, 0, Permissions.SEND_MESSAGES, 0),
    ]

    computed = Permissions.compute_channel_permissions(
        Permissions.VIEW_CHANNEL,
        overwrites,
        guild_id,
        member_id,
        [role_a, role_b],
    )
    assert (computed & Permissions.SEND_MESSAGES) != 0


def test_intents_and_capabilities():
    assert Intents.GUILDS == 1
    assert Intents.MESSAGE_CONTENT == 1 << 15
    assert Capabilities.CHANNEL_OBFUSCATION == 1 << 15
    assert Capabilities.IS_COMPONENTS_V2 == 1 << 16


def test_verify_interaction_signature():
    pk = "a" * 64
    sig = "b" * 128
    ts = "1700000000"
    body = b'{"type":1}'
    assert verify_interaction(pk, sig, ts, body) is False


def test_rate_limiter_and_normalize_route():
    norm = normalize_route("GET", "/channels/12345/messages/67890?limit=50")
    assert norm.template == "/channels/:major/messages/:id"
    assert norm.major == "channels/12345"

    limiter = RateLimiter()
    verdict, retry, _ = limiter.acquire("GET", "/channels/12345/messages", 1000)
    assert verdict == "go"

    verdict2, retry2, _ = limiter.acquire("GET", "/channels/12345/messages", 1005)
    assert verdict2 == "wait"

    limiter.on_response(
        "GET",
        "/channels/12345/messages",
        200,
        limit=5,
        remaining=4,
        reset_after_ms=1000,
        now_ms=1010,
    )
    verdict3, _, _ = limiter.acquire("GET", "/channels/12345/messages", 1015)
    assert verdict3 == "go"


def test_cache_and_nov16_obfuscation():
    cache = Cache()
    assert cache.stats().guilds == 0

    # Normal channel
    cache.update({
        "t": "CHANNEL_CREATE",
        "d": {"id": "101", "type": 0, "name": "chat", "flags": 0},
    })
    assert cache.is_channel_obfuscated("101") is False
    assert cache.get_channel("101") is not None

    # Obfuscated channel (flag bit 17 = 131072)
    cache.update({
        "t": "CHANNEL_CREATE",
        "d": {"id": "102", "type": 0, "name": "___hidden___", "flags": 131072},
    })
    assert cache.is_channel_obfuscated("102") is True
    assert cache.get_channel("102") is None
    assert cache.compute_member_permissions(1000, 102, 2000) == 0


def test_gateway_session_and_reconnection():
    gw = GatewaySession()
    assert gw.is_established() is False

    gw.on_hello(1000, 41250, 0.5)
    gw.on_ready("session_123", "wss://resume.discord.gg")
    gw.record_seq(50)
    assert gw.is_established() is True
    assert gw.seq() == 50

    action, seq = gw.poll_heartbeat(1000)
    assert action == "wait"

    # Plan reconnect
    plan, url, sess_id, seq_out, _ = gw.plan_reconnect("close", code=4000)
    assert plan == "resume"
    assert url == "wss://resume.discord.gg"
    assert sess_id == "session_123"
    assert seq_out == 50

    # Fatal close code
    fatal_plan, _, _, _, close_code = gw.plan_reconnect("close", code=4004)
    assert fatal_plan == "fatal"
    assert close_code == 4004


@pytest.mark.asyncio
async def test_rift_client_async():
    client = RiftClient("BOT_TOKEN")
    client.inject_mock_event(0, "MESSAGE_CREATE", {"content": "rift-py"})

    event = await client.next_event()
    assert event is not None
    assert event.shard_id == 0
    assert event.name == "MESSAGE_CREATE"
    assert event.data["content"] == "rift-py"

    client.update_presence(0, "online", "Python Bot", 0)
    await client.stop()
    next_event = await client.next_event()
    assert next_event is None
