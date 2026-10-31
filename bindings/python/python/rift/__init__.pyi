from typing import Any, Dict, List, Optional, Tuple, Union
from dataclasses import dataclass

DISCORD_EPOCH_MS: int

class Snowflake:
    @staticmethod
    def timestamp_ms(snowflake_id: Union[int, str]) -> int: ...
    @staticmethod
    def worker_id(snowflake_id: Union[int, str]) -> int: ...
    @staticmethod
    def process_id(snowflake_id: Union[int, str]) -> int: ...
    @staticmethod
    def increment(snowflake_id: Union[int, str]) -> int: ...
    @staticmethod
    def from_parts(timestamp_ms: int, worker_id: int, process_id: int, increment: int) -> int: ...
    @staticmethod
    def deconstruct(snowflake_id: Union[int, str]) -> Tuple[int, int, int, int]: ...

class Permissions:
    CREATE_INSTANT_INVITE: int
    KICK_MEMBERS: int
    BAN_MEMBERS: int
    ADMINISTRATOR: int
    MANAGE_CHANNELS: int
    MANAGE_GUILD: int
    ADD_REACTIONS: int
    VIEW_AUDIT_LOG: int
    PRIORITY_SPEAKER: int
    STREAM: int
    VIEW_CHANNEL: int
    SEND_MESSAGES: int
    SEND_TTS_MESSAGES: int
    MANAGE_MESSAGES: int
    EMBED_LINKS: int
    ATTACH_FILES: int
    READ_MESSAGE_HISTORY: int
    MENTION_EVERYONE: int
    USE_EXTERNAL_EMOJIS: int
    VIEW_GUILD_INSIGHTS: int
    CONNECT: int
    SPEAK: int
    MUTE_MEMBERS: int
    DEAFEN_MEMBERS: int
    MOVE_MEMBERS: int
    USE_VAD: int
    CHANGE_NICKNAME: int
    MANAGE_NICKNAMES: int
    MANAGE_ROLES: int
    MANAGE_WEBHOOKS: int
    MANAGE_GUILD_EXPRESSIONS: int
    USE_APPLICATION_COMMANDS: int
    REQUEST_TO_SPEAK: int
    MANAGE_EVENTS: int
    MANAGE_THREADS: int
    CREATE_PUBLIC_THREADS: int
    CREATE_PRIVATE_THREADS: int
    USE_EXTERNAL_STICKERS: int
    SEND_MESSAGES_IN_THREADS: int
    USE_EMBEDDED_ACTIVITIES: int
    MODERATE_MEMBERS: int
    VIEW_CREATOR_MONETIZATION_ANALYTICS: int
    USE_SOUNDBOARD: int
    CREATE_GUILD_EXPRESSIONS: int
    CREATE_EVENTS: int
    USE_EXTERNAL_SOUNDS: int
    SEND_VOICE_MESSAGES: int
    SET_VOICE_CHANNEL_STATUS: int
    SEND_POLLS: int
    USE_EXTERNAL_APPS: int
    PIN_MESSAGES: int
    BYPASS_SLOWMODE: int
    ALL: int

    @staticmethod
    def has_permission(permissions: Union[int, str], required: Union[int, str]) -> bool: ...
    @staticmethod
    def compute_channel_permissions(
        base_permissions: Union[int, str],
        overwrites: List[Tuple[int, int, int, int]],
        guild_id: Union[int, str],
        member_id: Union[int, str],
        member_roles: List[Union[int, str]],
    ) -> int: ...

class Intents:
    GUILDS: int
    GUILD_MEMBERS: int
    GUILD_MODERATION: int
    GUILD_EXPRESSIONS: int
    GUILD_INTEGRATIONS: int
    GUILD_WEBHOOKS: int
    GUILD_INVITES: int
    GUILD_VOICE_STATES: int
    GUILD_PRESENCES: int
    GUILD_MESSAGES: int
    GUILD_MESSAGE_REACTIONS: int
    GUILD_MESSAGE_TYPING: int
    DIRECT_MESSAGES: int
    DIRECT_MESSAGE_REACTIONS: int
    DIRECT_MESSAGE_TYPING: int
    MESSAGE_CONTENT: int
    GUILD_SCHEDULED_EVENTS: int
    AUTO_MODERATION_CONFIGURATION: int
    AUTO_MODERATION_EXECUTION: int
    GUILD_MESSAGE_POLLS: int
    DIRECT_MESSAGE_POLLS: int
    PRIVILEGED: int
    ALL_KNOWN: int

class Capabilities:
    NONE: int
    CHANNEL_OBFUSCATION: int
    IS_COMPONENTS_V2: int

def verify_interaction(
    public_key_hex: str,
    signature_hex: str,
    timestamp: str,
    body: Union[bytes, str],
) -> bool: ...

@dataclass
class NormalizedRoute:
    id: str
    template: str
    major: str

def normalize_route(method: str, path: str) -> NormalizedRoute: ...

class RateLimiter:
    def __init__(
        self,
        global_per_second: Optional[int] = None,
        invalid_limit: Optional[int] = None,
        invalid_trip_at: Optional[int] = None,
    ) -> None: ...
    def acquire(
        self, method: str, path: str, now_ms: Optional[int] = None
    ) -> Tuple[str, Optional[int], Optional[str]]: ...
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
    ) -> None: ...
    def invalid_requests(self, now_ms: Optional[int] = None) -> int: ...

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
    def __init__(
        self,
        max_messages_per_channel: Optional[int] = None,
        max_members_per_guild: Optional[int] = None,
        max_users: Optional[int] = None,
    ) -> None: ...
    def update(self, event_json_or_dict: Union[str, Dict[str, Any]]) -> None: ...
    def is_channel_obfuscated(self, channel_id: Union[int, str]) -> bool: ...
    def get_guild(self, guild_id: Union[int, str]) -> Optional[Dict[str, Any]]: ...
    def get_channel(self, channel_id: Union[int, str]) -> Optional[Dict[str, Any]]: ...
    def compute_member_permissions(
        self,
        guild_id: Union[int, str],
        channel_id: Union[int, str],
        user_id: Union[int, str],
    ) -> int: ...
    def stats(self) -> CacheStats: ...
    def clear(self) -> None: ...

class GatewaySession:
    def __init__(self) -> None: ...
    def reset(self) -> None: ...
    def on_hello(self, now_ms: int, interval_ms: int, jitter: float = 0.5) -> None: ...
    def on_ready(self, session_id: str, resume_gateway_url: str) -> None: ...
    def record_seq(self, seq: int) -> None: ...
    def seq(self) -> Optional[int]: ...
    def is_established(self) -> bool: ...
    def poll_heartbeat(self, now_ms: int) -> Tuple[str, Optional[int]]: ...
    def plan_reconnect(
        self, why: str, code: Optional[int] = None, resumable: Optional[bool] = None
    ) -> Tuple[str, Optional[str], Optional[str], Optional[int], Optional[int]]: ...

@dataclass
class ClientEvent:
    shard_id: int
    name: str
    data: Any

class RiftClient:
    token: str
    intents: int
    capabilities: int
    def __init__(
        self,
        token: str,
        intents: Optional[int] = None,
        capabilities: Optional[int] = None,
    ) -> None: ...
    def inject_mock_event(self, shard_id: int, name: str, payload: Union[str, Dict[str, Any]]) -> None: ...
    async def next_event(self) -> Optional[ClientEvent]: ...
    def send_command(self, shard_id: int, opcode: int, payload: Any) -> None: ...
    def update_presence(
        self,
        shard_id: int,
        status: str,
        activity_name: Optional[str] = None,
        activity_type: int = 0,
    ) -> None: ...
    async def stop(self) -> None: ...
