"""
Rift Discord Engine - Python Bindings.

High-performance, memory-safe, spec-compliant Discord engine written in Rust.
"""

from __future__ import annotations

import asyncio
import json
import time
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Tuple, Union

# Attempt to load compiled native PyO3 C-extension
try:
    from . import _rift
except ImportError:
    try:
        import _rift
    except ImportError:
        _rift = None

DISCORD_EPOCH_MS = 1420070400000


# ---------------------------------------------------------------------------
# 1. Snowflake
# ---------------------------------------------------------------------------
class Snowflake:
    """Discord snowflake ID decoding, inspection, and generation."""

    @staticmethod
    def timestamp_ms(snowflake_id: Union[int, str]) -> int:
        n = int(snowflake_id)
        if _rift is not None:
            return _rift.Snowflake.timestamp_ms(n)
        return (n >> 22) + DISCORD_EPOCH_MS

    @staticmethod
    def worker_id(snowflake_id: Union[int, str]) -> int:
        n = int(snowflake_id)
        if _rift is not None:
            return _rift.Snowflake.worker_id(n)
        return (n & 0x3E0000) >> 17

    @staticmethod
    def process_id(snowflake_id: Union[int, str]) -> int:
        n = int(snowflake_id)
        if _rift is not None:
            return _rift.Snowflake.process_id(n)
        return (n & 0x1F000) >> 12

    @staticmethod
    def increment(snowflake_id: Union[int, str]) -> int:
        n = int(snowflake_id)
        if _rift is not None:
            return _rift.Snowflake.increment(n)
        return n & 0xFFF

    @staticmethod
    def from_parts(timestamp_ms: int, worker_id: int, process_id: int, increment: int) -> int:
        if _rift is not None:
            return _rift.Snowflake.from_parts(timestamp_ms, worker_id, process_id, increment)
        ts_offset = max(0, timestamp_ms - DISCORD_EPOCH_MS)
        return (
            (ts_offset << 22)
            | ((worker_id & 0x1F) << 17)
            | ((process_id & 0x1F) << 12)
            | (increment & 0xFFF)
        )

    @staticmethod
    def deconstruct(snowflake_id: Union[int, str]) -> Tuple[int, int, int, int]:
        n = int(snowflake_id)
        if _rift is not None:
            return _rift.Snowflake.deconstruct(n)
        return (
            Snowflake.timestamp_ms(n),
            Snowflake.worker_id(n),
            Snowflake.process_id(n),
            Snowflake.increment(n),
        )


# ---------------------------------------------------------------------------
# 2. Permissions
# ---------------------------------------------------------------------------
class Permissions:
    """Discord permission bitflags and channel overwrite mathematics."""

    CREATE_INSTANT_INVITE = 1 << 0
    KICK_MEMBERS = 1 << 1
    BAN_MEMBERS = 1 << 2
    ADMINISTRATOR = 1 << 3
    MANAGE_CHANNELS = 1 << 4
    MANAGE_GUILD = 1 << 5
    ADD_REACTIONS = 1 << 6
    VIEW_AUDIT_LOG = 1 << 7
    PRIORITY_SPEAKER = 1 << 8
    STREAM = 1 << 9
    VIEW_CHANNEL = 1 << 10
    SEND_MESSAGES = 1 << 11
    SEND_TTS_MESSAGES = 1 << 12
    MANAGE_MESSAGES = 1 << 13
    EMBED_LINKS = 1 << 14
    ATTACH_FILES = 1 << 15
    READ_MESSAGE_HISTORY = 1 << 16
    MENTION_EVERYONE = 1 << 17
    USE_EXTERNAL_EMOJIS = 1 << 18
    VIEW_GUILD_INSIGHTS = 1 << 19
    CONNECT = 1 << 20
    SPEAK = 1 << 21
    MUTE_MEMBERS = 1 << 22
    DEAFEN_MEMBERS = 1 << 23
    MOVE_MEMBERS = 1 << 24
    USE_VAD = 1 << 25
    CHANGE_NICKNAME = 1 << 26
    MANAGE_NICKNAMES = 1 << 27
    MANAGE_ROLES = 1 << 28
    MANAGE_WEBHOOKS = 1 << 29
    MANAGE_GUILD_EXPRESSIONS = 1 << 30
    USE_APPLICATION_COMMANDS = 1 << 31
    REQUEST_TO_SPEAK = 1 << 32
    MANAGE_EVENTS = 1 << 33
    MANAGE_THREADS = 1 << 34
    CREATE_PUBLIC_THREADS = 1 << 35
    CREATE_PRIVATE_THREADS = 1 << 36
    USE_EXTERNAL_STICKERS = 1 << 37
    SEND_MESSAGES_IN_THREADS = 1 << 38
    USE_EMBEDDED_ACTIVITIES = 1 << 39
    MODERATE_MEMBERS = 1 << 40
    VIEW_CREATOR_MONETIZATION_ANALYTICS = 1 << 41
    USE_SOUNDBOARD = 1 << 42
    CREATE_GUILD_EXPRESSIONS = 1 << 43
    CREATE_EVENTS = 1 << 44
    USE_EXTERNAL_SOUNDS = 1 << 45
    SEND_VOICE_MESSAGES = 1 << 46
    SET_VOICE_CHANNEL_STATUS = 1 << 48
    SEND_POLLS = 1 << 49
    USE_EXTERNAL_APPS = 1 << 50
    PIN_MESSAGES = 1 << 51
    BYPASS_SLOWMODE = 1 << 52
    ALL = (1 << 53) - 1 - (1 << 47)

    @staticmethod
    def has_permission(permissions: Union[int, str], required: Union[int, str]) -> bool:
        p = int(permissions)
        r = int(required)
        if _rift is not None:
            return _rift.Permissions.has_permission(p, r)
        if (p & Permissions.ADMINISTRATOR) == Permissions.ADMINISTRATOR:
            return True
        return (p & r) == r

    @staticmethod
    def compute_channel_permissions(
        base_permissions: Union[int, str],
        overwrites: List[Tuple[int, int, int, int]],  # (id, kind: 0=role 1=member, allow, deny)
        guild_id: Union[int, str],
        member_id: Union[int, str],
        member_roles: List[Union[int, str]],
    ) -> int:
        p = int(base_permissions)
        g_id = int(guild_id)
        m_id = int(member_id)
        roles = [int(r) for r in member_roles]

        if _rift is not None:
            return _rift.Permissions.compute_channel_permissions(p, overwrites, g_id, m_id, roles)

        if (p & Permissions.ADMINISTRATOR) == Permissions.ADMINISTRATOR:
            return Permissions.ALL

        # 1. @everyone overwrite
        for o_id, kind, allow, deny in overwrites:
            if kind == 0 and o_id == g_id:
                p = (p & ~deny) | allow
                break

        # 2. Role overwrites (allow beats deny across roles)
        role_allow = 0
        role_deny = 0
        roles_set = set(roles)
        for o_id, kind, allow, deny in overwrites:
            if kind == 0 and o_id in roles_set:
                role_allow |= allow
                role_deny |= deny
        p = (p & ~role_deny) | role_allow

        # 3. Member overwrite
        for o_id, kind, allow, deny in overwrites:
            if kind == 1 and o_id == m_id:
                p = (p & ~deny) | allow
                break

        # 4. Implicit rules
        if (p & Permissions.VIEW_CHANNEL) == 0:
            return 0
        if (p & Permissions.SEND_MESSAGES) == 0:
            dependent = (
                Permissions.MENTION_EVERYONE
                | Permissions.SEND_TTS_MESSAGES
                | Permissions.ATTACH_FILES
                | Permissions.EMBED_LINKS
            )
            p &= ~dependent

        return p


# ---------------------------------------------------------------------------
# 3. Intents & Capabilities
# ---------------------------------------------------------------------------
class Intents:
    GUILDS = 1 << 0
    GUILD_MEMBERS = 1 << 1
    GUILD_MODERATION = 1 << 2
    GUILD_EXPRESSIONS = 1 << 3
    GUILD_INTEGRATIONS = 1 << 4
    GUILD_WEBHOOKS = 1 << 5
    GUILD_INVITES = 1 << 6
    GUILD_VOICE_STATES = 1 << 7
    GUILD_PRESENCES = 1 << 8
    GUILD_MESSAGES = 1 << 9
    GUILD_MESSAGE_REACTIONS = 1 << 10
    GUILD_MESSAGE_TYPING = 1 << 11
    DIRECT_MESSAGES = 1 << 12
    DIRECT_MESSAGE_REACTIONS = 1 << 13
    DIRECT_MESSAGE_TYPING = 1 << 14
    MESSAGE_CONTENT = 1 << 15
    GUILD_SCHEDULED_EVENTS = 1 << 16
    AUTO_MODERATION_CONFIGURATION = 1 << 20
    AUTO_MODERATION_EXECUTION = 1 << 21
    GUILD_MESSAGE_POLLS = 1 << 24
    DIRECT_MESSAGE_POLLS = 1 << 25
    PRIVILEGED = (1 << 1) | (1 << 8) | (1 << 15)
    ALL_KNOWN = (
        (1 << 0)
        | (1 << 1)
        | (1 << 2)
        | (1 << 3)
        | (1 << 4)
        | (1 << 5)
        | (1 << 6)
        | (1 << 7)
        | (1 << 8)
        | (1 << 9)
        | (1 << 10)
        | (1 << 11)
        | (1 << 12)
        | (1 << 13)
        | (1 << 14)
        | (1 << 15)
        | (1 << 16)
        | (1 << 20)
        | (1 << 21)
        | (1 << 24)
        | (1 << 25)
    )


class Capabilities:
    NONE = 0
    CHANNEL_OBFUSCATION = 1 << 15
    IS_COMPONENTS_V2 = 1 << 16


# ---------------------------------------------------------------------------
# 4. verify_interaction
# ---------------------------------------------------------------------------
def verify_interaction(
    public_key_hex: str,
    signature_hex: str,
    timestamp: str,
    body: Union[bytes, str],
) -> bool:
    """Verifies Discord interaction webhook Ed25519 signature."""
    b = body.encode("utf-8") if isinstance(body, str) else body
    if _rift is not None:
        return _rift.verify_interaction(public_key_hex, signature_hex, timestamp, b)

    try:
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

        pub_bytes = bytes.fromhex(public_key_hex)
        sig_bytes = bytes.fromhex(signature_hex)
        if len(pub_bytes) != 32 or len(sig_bytes) != 64:
            return False
        key = Ed25519PublicKey.from_public_bytes(pub_bytes)
        key.verify(sig_bytes, timestamp.encode("utf-8") + b)
        return True
    except Exception:
        return False


# ---------------------------------------------------------------------------
# 5. RateLimiter & normalize_route
# ---------------------------------------------------------------------------
@dataclass
class NormalizedRoute:
    id: str
    template: str
    major: str


def normalize_route(method: str, path: str) -> NormalizedRoute:
    """Normalizes an HTTP method and path into Discord rate limit bucket keys."""
    if _rift is not None:
        r = _rift.normalize_route(method, path)
        return NormalizedRoute(id=r.id, template=r.template, major=r.major)

    p = path.split("?")[0]
    segs = [s for s in p.split("/") if s]

    if segs and segs[0] == "interactions":
        rest = "".join(f"/{s}" for s in segs[3:])
        template = f"/interactions/:id/:token{rest}"
        return NormalizedRoute(
            id=f"{method.upper()} {template}",
            template=template,
            major="interactions",
        )

    out = []
    major = ""
    i = 0
    while i < len(segs):
        s = segs[i]
        prev = segs[i - 1] if i > 0 else ""
        is_snow = s.isdigit()
        if is_snow and prev in ("channels", "guilds"):
            if not major:
                major = f"{prev}/{s}"
            out.append(":major")
        elif is_snow and prev == "webhooks":
            m = f"webhooks/{s}"
            if i + 1 < len(segs):
                m += f"/{segs[i + 1]}"
                i += 1
            if not major:
                major = m
            out.append(":major")
        elif is_snow:
            out.append(":id")
        elif prev == "reactions":
            out.append(":emoji")
        elif prev == "invites":
            out.append(":code")
        else:
            out.append(s)
        i += 1

    template = "/" + "/".join(out)
    return NormalizedRoute(
        id=f"{method.upper()} {template}",
        template=template,
        major=major,
    )


class RateLimiter:
    """Discord REST bucket rate limiter."""

    def __init__(
        self,
        global_per_second: Optional[int] = None,
        invalid_limit: Optional[int] = None,
        invalid_trip_at: Optional[int] = None,
    ):
        if _rift is not None:
            self._inner = _rift.RateLimiter(global_per_second, invalid_limit, invalid_trip_at)
        else:
            self._inner = None
        self._buckets: Dict[str, Dict[str, Any]] = {}
        self._invalid_log: List[int] = []

    def acquire(
        self, method: str, path: str, now_ms: Optional[int] = None
    ) -> Tuple[str, Optional[int], Optional[str]]:
        now = now_ms if now_ms is not None else int(time.time() * 1000)
        if self._inner is not None:
            return self._inner.acquire(method, path, now)

        norm = normalize_route(method, path)
        key = f"{norm.template}|{norm.major}"
        b = self._buckets.get(key)
        if not b:
            self._buckets[key] = {"limit": 1, "remaining": 0, "reset_at": now + 30000, "probing": True}
            return ("go", None, None)

        if b.get("probing"):
            return ("wait", 50, None)

        if now >= b["reset_at"]:
            b["remaining"] = b["limit"]

        if b["remaining"] > 0:
            b["remaining"] -= 1
            return ("go", None, None)

        wait = max(0, b["reset_at"] - now)
        return ("wait", wait, None)

    def on_response(
        self,
        method: str,
        path: str,
        status: int,
        limit: Optional[int] = None,
        remaining: Optional[int] = None,
        reset_after_ms: Optional[int] = None,
        bucket: Optional[str] = None,
        scope: Optional[str] = None,
        now_ms: Optional[int] = None,
    ) -> None:
        now = now_ms if now_ms is not None else int(time.time() * 1000)
        if self._inner is not None:
            self._inner.on_response(
                method, path, status, limit, remaining, reset_after_ms, bucket, scope, now
            )
            return

        norm = normalize_route(method, path)
        key = f"{norm.template}|{norm.major}"
        self._buckets[key] = {
            "limit": limit or 1,
            "remaining": remaining or 1,
            "reset_at": now + (reset_after_ms or 1000),
            "probing": False,
        }
        if status in (401, 403, 429) and scope != "shared":
            self._invalid_log.append(now)

    def invalid_requests(self, now_ms: Optional[int] = None) -> int:
        now = now_ms if now_ms is not None else int(time.time() * 1000)
        if self._inner is not None:
            return self._inner.invalid_requests(now)
        cutoff = now - 600000
        self._invalid_log = [t for t in self._invalid_log if t >= cutoff]
        return len(self._invalid_log)


# ---------------------------------------------------------------------------
# 6. Cache
# ---------------------------------------------------------------------------
@dataclass
class CacheStats:
    guilds: int
    visible_channels: int
    obfuscated_channels: int
    roles: int
    members: int
    users: int
    messages: int


class Cache:
    """In-memory state cache with Nov 16 channel obfuscation safety."""

    def __init__(
        self,
        max_messages_per_channel: Optional[int] = None,
        max_members_per_guild: Optional[int] = None,
        max_users: Optional[int] = None,
    ):
        if _rift is not None:
            self._inner = _rift.Cache(
                max_messages_per_channel, max_members_per_guild, max_users
            )
        else:
            self._inner = None
        self._guilds: Dict[str, Any] = {}
        self._channels: Dict[str, Any] = {}
        self._obfuscated: set[str] = set()

    def update(self, event_json_or_dict: Union[str, Dict[str, Any]]) -> None:
        payload = (
            json.dumps(event_json_or_dict)
            if isinstance(event_json_or_dict, dict)
            else event_json_or_dict
        )
        if self._inner is not None:
            self._inner.update(payload)
            return

        d = json.loads(payload)
        t = d.get("t") or d.get("name")
        body = d.get("d") or d.get("data") or d

        if t == "GUILD_CREATE":
            self._guilds[str(body["id"])] = body
            for ch in body.get("channels", []):
                self._insert_channel(ch)
        elif t in ("CHANNEL_CREATE", "CHANNEL_UPDATE"):
            self._insert_channel(body)
        elif t == "CHANNEL_DELETE":
            cid = str(body["id"])
            self._channels.pop(cid, None)
            self._obfuscated.discard(cid)

    def _insert_channel(self, ch: Dict[str, Any]) -> None:
        cid = str(ch["id"])
        if (ch.get("flags", 0) & (1 << 17)) != 0:
            self._obfuscated.add(cid)
            sanitized = dict(ch)
            sanitized["name"] = None
            sanitized["topic"] = None
            sanitized["permission_overwrites"] = []
            self._channels[cid] = sanitized
        else:
            self._obfuscated.discard(cid)
            self._channels[cid] = ch

    def is_channel_obfuscated(self, channel_id: Union[int, str]) -> bool:
        cid = str(channel_id)
        if self._inner is not None:
            return self._inner.is_channel_obfuscated(int(cid))
        return cid in self._obfuscated

    def get_guild(self, guild_id: Union[int, str]) -> Optional[Dict[str, Any]]:
        gid = str(guild_id)
        if self._inner is not None:
            res = self._inner.get_guild(int(gid))
            return json.loads(res) if res else None
        return self._guilds.get(gid)

    def get_channel(self, channel_id: Union[int, str]) -> Optional[Dict[str, Any]]:
        cid = str(channel_id)
        if self.is_channel_obfuscated(cid):
            return None
        if self._inner is not None:
            res = self._inner.get_channel(int(cid))
            return json.loads(res) if res else None
        return self._channels.get(cid)

    def compute_member_permissions(
        self,
        guild_id: Union[int, str],
        channel_id: Union[int, str],
        user_id: Union[int, str],
    ) -> int:
        if self.is_channel_obfuscated(channel_id):
            return 0
        if self._inner is not None:
            return self._inner.compute_member_permissions(int(guild_id), int(channel_id), int(user_id))
        ch = self._channels.get(str(channel_id))
        if not ch:
            return 0
        return Permissions.compute_channel_permissions(
            Permissions.ALL,
            [
                (int(o["id"]), o.get("type", 0), int(o.get("allow", 0)), int(o.get("deny", 0)))
                for o in ch.get("permission_overwrites", [])
            ],
            int(guild_id),
            int(user_id),
            [],
        )

    def stats(self) -> CacheStats:
        if self._inner is not None:
            g, vc, oc, r, m, u, msg = self._inner.stats()
            return CacheStats(g, vc, oc, r, m, u, msg)
        return CacheStats(
            guilds=len(self._guilds),
            visible_channels=max(0, len(self._channels) - len(self._obfuscated)),
            obfuscated_channels=len(self._obfuscated),
            roles=0,
            members=0,
            users=0,
            messages=0,
        )

    def clear(self) -> None:
        if self._inner is not None:
            self._inner.clear()
        self._guilds.clear()
        self._channels.clear()
        self._obfuscated.clear()


# ---------------------------------------------------------------------------
# 7. GatewaySession
# ---------------------------------------------------------------------------
class GatewaySession:
    """Sans-I/O Gateway v10 state machine."""

    def __init__(self):
        if _rift is not None:
            self._inner = _rift.GatewaySession()
        else:
            self._inner = None
        self.reset()

    def reset(self) -> None:
        if self._inner is not None:
            self._inner.reset()
        self.interval_ms = 0
        self.next_due_ms = 0
        self.awaiting_ack = False
        self.started = False
        self.sequence: Optional[int] = None
        self.session_id: Optional[str] = None
        self.resume_url: Optional[str] = None

    def on_hello(self, now_ms: int, interval_ms: int, jitter: float = 0.5) -> None:
        if self._inner is not None:
            self._inner.on_hello(now_ms, interval_ms, jitter)
            return
        j = min(1.0, max(0.0, jitter))
        self.interval_ms = interval_ms
        self.next_due_ms = now_ms + int(interval_ms * j)
        self.awaiting_ack = False
        self.started = True

    def on_ready(self, session_id: str, resume_gateway_url: str) -> None:
        if self._inner is not None:
            self._inner.on_ready(session_id, resume_gateway_url)
            return
        self.session_id = session_id
        self.resume_url = resume_gateway_url

    def record_seq(self, seq: int) -> None:
        if self._inner is not None:
            self._inner.record_seq(seq)
            return
        self.sequence = seq

    def seq(self) -> Optional[int]:
        if self._inner is not None:
            return self._inner.seq()
        return self.sequence

    def is_established(self) -> bool:
        if self._inner is not None:
            return self._inner.is_established()
        return self.session_id is not None

    def poll_heartbeat(self, now_ms: int) -> Tuple[str, Optional[int]]:
        """Returns (action: "wait"|"send"|"zombie", seq: Optional[int])."""
        if self._inner is not None:
            return self._inner.poll_heartbeat(now_ms)
        if not self.started or now_ms < self.next_due_ms:
            return ("wait", None)
        if self.awaiting_ack:
            return ("zombie", None)
        self.awaiting_ack = True
        self.next_due_ms = now_ms + self.interval_ms
        return ("send", self.sequence)

    def plan_reconnect(
        self, why: str, code: Optional[int] = None, resumable: Optional[bool] = None
    ) -> Tuple[str, Optional[str], Optional[str], Optional[int], Optional[int]]:
        """Returns (plan: "resume"|"identify"|"fatal", url, session_id, seq, close_code)."""
        if self._inner is not None:
            return self._inner.plan_reconnect(why, code, resumable)
        if code in (4004, 4010, 4011, 4012, 4013, 4014):
            return ("fatal", None, None, None, code)
        if why == "invalid_session" and not resumable:
            self.reset()
            return ("identify", None, None, None, None)
        if self.session_id and self.resume_url and self.sequence is not None:
            return ("resume", self.resume_url, self.session_id, self.sequence, None)
        self.reset()
        return ("identify", None, None, None, None)


# ---------------------------------------------------------------------------
# 8. RiftClient
# ---------------------------------------------------------------------------
@dataclass
class ClientEvent:
    shard_id: int
    name: str
    data: Any


class RiftClient:
    """High-level asynchronous Rift Client."""

    def __init__(
        self,
        token: str,
        intents: Optional[int] = None,
        capabilities: Optional[int] = None,
    ):
        self.token = token
        self.intents = intents or Intents.ALL_KNOWN
        self.capabilities = capabilities or Capabilities.NONE
        self._queue: asyncio.Queue[Optional[ClientEvent]] = asyncio.Queue()
        self._stopped = False

    def inject_mock_event(self, shard_id: int, name: str, payload: Union[str, Dict[str, Any]]) -> None:
        data = json.loads(payload) if isinstance(payload, str) else payload
        self._queue.put_nowait(ClientEvent(shard_id, name, data))

    async def next_event(self) -> Optional[ClientEvent]:
        if self._stopped and self._queue.empty():
            return None
        return await self._queue.get()

    def send_command(self, shard_id: int, opcode: int, payload: Any) -> None:
        pass

    def update_presence(
        self,
        shard_id: int,
        status: str,
        activity_name: Optional[str] = None,
        activity_type: int = 0,
    ) -> None:
        payload = {
            "status": status,
            "since": None,
            "afk": False,
            "activities": (
                [{"name": activity_name, "type": activity_type}] if activity_name else []
            ),
        }
        self.send_command(shard_id, 3, payload)

    async def stop(self) -> None:
        self._stopped = True
        await self._queue.put(None)


__all__ = [
    "Snowflake",
    "Permissions",
    "Intents",
    "Capabilities",
    "verify_interaction",
    "normalize_route",
    "NormalizedRoute",
    "RateLimiter",
    "Cache",
    "CacheStats",
    "GatewaySession",
    "RiftClient",
    "ClientEvent",
]
