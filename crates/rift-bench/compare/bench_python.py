#!/usr/bin/env python3
"""
Cross-Library Comparative Benchmark Runner: Python (CPython / discord.py model)

Implements the exact algorithmic pipeline used by discord.py:
1. Wire transport: WebSocket JSON framing (uncompressed text or zlib) over loopback TCP
2. Eager dynamic JSON parsing via CPython json.loads into dict object graphs
3. State caching into dynamic dict with unbounded object reference retention
4. Snowflake ID decode via int parsing and bitshift arithmetic
5. 52-bit Discord permission resolution via int bitwise operations
6. REST route normalization via sequential re.sub regex chain replacements

Outputs a machine-readable JSON metrics object to stdout.
Requires ONLY the Python standard library (no third-party packages needed).
"""

import sys
import time
import json
import re
import argparse
import socket
import urllib.parse
import base64
import os
import struct
import tracemalloc

DISCORD_EPOCH = 1420070400000

def parse_args():
    parser = argparse.ArgumentParser(description="Python discord.py comparison benchmark")
    parser.add_argument("--events", type=int, default=10000, help="Number of dispatch events")
    parser.add_argument("--guild-members", type=int, default=100, help="Members in initial guild")
    parser.add_argument("--connect", type=str, default=None, help="WebSocket URL of MockDiscordServer")
    return parser.parse_args()

def snowflake_id(n):
    return str(1100000000000000000 + n * 1337001)

def parse_snowflake(s):
    val = int(s)
    timestamp = (val >> 22) + DISCORD_EPOCH
    worker_id = (val & 0x3E0000) >> 17
    process_id = (val & 0x1F000) >> 12
    increment = val & 0xFFF
    return timestamp, worker_id, process_id, increment

# discord.py REST route normalization regexes
ROUTE_CHANNELS = re.compile(r'/channels/\d+')
ROUTE_GUILDS = re.compile(r'/guilds/\d+')
ROUTE_WEBHOOKS = re.compile(r'/webhooks/\d+/[^/]+')
ROUTE_MESSAGES = re.compile(r'/messages/\d+')

def normalize_route(method, path):
    cleaned = path.split('?')[0]
    cleaned = ROUTE_CHANNELS.sub('/channels/:id', cleaned)
    cleaned = ROUTE_GUILDS.sub('/guilds/:id', cleaned)
    cleaned = ROUTE_WEBHOOKS.sub('/webhooks/:id/:token', cleaned)
    cleaned = ROUTE_MESSAGES.sub('/messages/:id', cleaned)
    return f"{method.upper()} {cleaned}"

def resolve_permissions(member_roles, role_map, overwrites=None):
    perms = 0
    for rid in member_roles:
        r = role_map.get(rid)
        if r:
            perms |= int(r.get("permissions", "0"))

    # Administrator check (1 << 3)
    if (perms & 8) == 8:
        return ~0

    # Overwrites
    if overwrites:
        allow = 0
        deny = 0
        for ow in overwrites:
            allow |= int(ow.get("allow", "0"))
            deny |= int(ow.get("deny", "0"))
        perms = (perms & ~deny) | allow

    return perms

def generate_payload(seq, i):
    return json.dumps({
        "op": 0,
        "s": seq,
        "t": "MESSAGE_CREATE",
        "d": {
            "id": snowflake_id(i),
            "channel_id": snowflake_id(7),
            "guild_id": snowflake_id(3),
            "type": 0,
            "content": f"message {i}: the quick brown fox jumps over the lazy dog {(i * 7919) % 10007}",
            "author": {
                "id": snowflake_id(i % 500),
                "username": f"user{i % 500}",
                "discriminator": "0",
                "avatar": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
                "global_name": f"User {i % 500}",
                "public_flags": 0
            },
            "member": {
                "roles": [snowflake_id(11), snowflake_id(12)],
                "joined_at": "2024-03-01T12:00:00.000000+00:00",
                "nick": None,
                "flags": 0,
                "mute": False,
                "deaf": False
            },
            "timestamp": "2026-10-30T06:44:00.123000+00:00",
            "edited_timestamp": None,
            "tts": False,
            "mention_everyone": False,
            "mentions": [],
            "mention_roles": [],
            "attachments": [],
            "embeds": [],
            "pinned": False,
            "components": [],
            "flags": 0
        }
    })

def generate_guild_create(guild_members):
    members = []
    for i in range(guild_members):
        members.append({
            "user": {
                "id": snowflake_id(i),
                "username": f"user{i}",
                "avatar": "a1b2c3d4e5f60718293a4b5c6d7e8f90",
                "bot": False
            },
            "roles": [snowflake_id(11), snowflake_id(i % 40)],
            "joined_at": "2024-03-01T12:00:00.000000+00:00",
            "nick": None,
            "flags": 0,
            "mute": False,
            "deaf": False
        })

    return json.dumps({
        "op": 0,
        "s": 2,
        "t": "GUILD_CREATE",
        "d": {
            "id": snowflake_id(3),
            "name": "bench-guild",
            "member_count": guild_members,
            "members": members,
            "channels": [
                {
                    "id": snowflake_id(7),
                    "type": 0,
                    "guild_id": snowflake_id(3),
                    "name": "general",
                    "position": 0,
                    "permission_overwrites": [
                        {"id": snowflake_id(11), "type": 0, "allow": "2048", "deny": "0"}
                    ]
                }
            ],
            "roles": [
                {
                    "id": snowflake_id(11),
                    "name": "bench-role",
                    "color": 0,
                    "hoist": False,
                    "position": 1,
                    "permissions": "104320577",
                    "managed": False,
                    "mentionable": False
                },
                {
                    "id": snowflake_id(12),
                    "name": "member-role",
                    "color": 0,
                    "hoist": False,
                    "position": 2,
                    "permissions": "2048",
                    "managed": False,
                    "mentionable": False
                }
            ]
        }
    })

def send_client_frame(sock, text):
    payload = text.encode("utf-8")
    length = len(payload)
    mask_key = os.urandom(4)
    if length < 126:
        header = struct.pack("!BB", 0x81, 0x80 | length)
    elif length < 65536:
        header = struct.pack("!BBH", 0x81, 0x80 | 126, length)
    else:
        header = struct.pack("!BBQ", 0x81, 0x80 | 127, length)

    masked = bytearray(length)
    for i in range(length):
        masked[i] = payload[i] ^ mask_key[i % 4]

    sock.sendall(header + mask_key + masked)

def send_client_close(sock):
    mask_key = os.urandom(4)
    code = struct.pack("!H", 1000)
    masked = bytearray([code[0] ^ mask_key[0], code[1] ^ mask_key[1]])
    try:
        sock.sendall(struct.pack("!BB", 0x88, 0x80 | 2) + mask_key + masked)
    except Exception:
        pass

class WebSocketReader:
    def __init__(self, sock, initial=b""):
        self.sock = sock
        self.buf = bytearray(initial)

    def read_exact(self, n):
        while len(self.buf) < n:
            chunk = self.sock.recv(65536)
            if not chunk:
                return None
            self.buf.extend(chunk)
        res = bytes(self.buf[:n])
        del self.buf[:n]
        return res

    def next_frame(self):
        while True:
            hdr = self.read_exact(2)
            if not hdr:
                return None
            b1, b2 = hdr[0], hdr[1]
            opcode = b1 & 0x0F
            masked = (b2 & 0x80) != 0
            length = b2 & 0x7F

            if length == 126:
                ext = self.read_exact(2)
                if not ext:
                    return None
                length = struct.unpack("!H", ext)[0]
            elif length == 127:
                ext = self.read_exact(8)
                if not ext:
                    return None
                length = struct.unpack("!Q", ext)[0]

            mask_key = self.read_exact(4) if masked else None
            payload = self.read_exact(length)
            if payload is None:
                return None

            if masked and mask_key:
                unmasked = bytearray(length)
                for i in range(length):
                    unmasked[i] = payload[i] ^ mask_key[i % 4]
                payload = bytes(unmasked)

            if opcode == 1:  # Text frame
                return payload.decode("utf-8", errors="replace")
            elif opcode == 8:  # Close frame
                return None
            elif opcode == 9:  # Ping
                mask = os.urandom(4)
                pong = struct.pack("!BB", 0x8A, 0x80) + mask
                try:
                    self.sock.sendall(pong)
                except Exception:
                    pass

def run_websocket(opts):
    tracemalloc.start()
    u = urllib.parse.urlparse(opts.connect)
    host = u.hostname or "127.0.0.1"
    port = u.port or 80
    path = u.path or "/"
    if u.query:
        path += "?" + u.query

    s = socket.create_connection((host, port), timeout=15)
    key = base64.b64encode(os.urandom(16)).decode("ascii")
    upgrade_req = (
        f"GET {path} HTTP/1.1\r\n"
        f"Host: {host}:{port}\r\n"
        f"Upgrade: websocket\r\n"
        f"Connection: Upgrade\r\n"
        f"Sec-WebSocket-Key: {key}\r\n"
        f"Sec-WebSocket-Version: 13\r\n\r\n"
    )
    s.sendall(upgrade_req.encode("ascii"))

    buf = bytearray()
    while b"\r\n\r\n" not in buf:
        chunk = s.recv(4096)
        if not chunk:
            raise ConnectionError("Connection closed during WebSocket handshake")
        buf.extend(chunk)

    end_idx = buf.find(b"\r\n\r\n") + 4
    reader = WebSocketReader(s, initial=buf[end_idx:])

    latencies = []
    message_cache = {}
    role_map = {}
    events_ingested = 0
    burst_start = None
    burst_duration = 1.0

    while True:
        msg = reader.next_frame()
        if msg is None:
            break

        t0 = time.perf_counter()
        data = json.loads(msg)

        op = data.get("op")
        if op == 10:
            # HELLO -> IDENTIFY
            identify_payload = {
                "op": 2,
                "d": {
                    "token": "rift-bench-token",
                    "intents": 3276799,
                    "properties": {"$os": "windows", "$browser": "discord.py", "$device": "discord.py"}
                }
            }
            send_client_frame(s, json.dumps(identify_payload))
            continue

        t = data.get("t")
        if t == "READY":
            burst_start = time.perf_counter()
            continue

        if t == "GUILD_CREATE":
            guild = data.get("d", {})
            for r in guild.get("roles", []):
                role_map[r["id"]] = r
        elif t == "MESSAGE_CREATE":
            m_obj = data.get("d", {})
            message_cache[m_obj["id"]] = m_obj
            parse_snowflake(m_obj["id"])
            member = m_obj.get("member")
            if member and "roles" in member:
                resolve_permissions(member["roles"], role_map)
            normalize_route("POST", f"/channels/{m_obj['channel_id']}/messages/{m_obj['id']}")

        elapsed_us = (time.perf_counter() - t0) * 1_000_000.0
        latencies.append(elapsed_us)
        events_ingested += 1

        if events_ingested >= opts.events:
            burst_duration = (time.perf_counter() - burst_start) * 1000.0 if burst_start else 1.0
            send_client_close(s)
            break

    s.close()
    current_mem, peak_mem = tracemalloc.get_traced_memory()
    tracemalloc.stop()

    latencies.sort()
    p50 = latencies[int(len(latencies) * 0.50)] if latencies else 0
    p95 = latencies[int(len(latencies) * 0.95)] if latencies else 0
    p99 = latencies[int(len(latencies) * 0.99)] if latencies else 0
    eps = (events_ingested / (burst_duration / 1000.0)) if burst_duration > 0 else 0

    # Micro-benchmarks
    MICRO_ITERS = 5000
    sample_snowflake = snowflake_id(12345)
    t_start = time.perf_counter()
    for _ in range(MICRO_ITERS):
        parse_snowflake(sample_snowflake)
    micro_snowflake_ns = ((time.perf_counter() - t_start) / MICRO_ITERS) * 1_000_000_000.0

    sample_roles = [snowflake_id(11), snowflake_id(12)]
    t_start = time.perf_counter()
    for _ in range(MICRO_ITERS):
        resolve_permissions(sample_roles, role_map)
    micro_perm_ns = ((time.perf_counter() - t_start) / MICRO_ITERS) * 1_000_000_000.0

    sample_route = f"/channels/{snowflake_id(123)}/messages/{snowflake_id(456)}"
    t_start = time.perf_counter()
    for _ in range(MICRO_ITERS):
        normalize_route("POST", sample_route)
    micro_route_ns = ((time.perf_counter() - t_start) / MICRO_ITERS) * 1_000_000_000.0

    result = {
        "runtime": "Python (CPython / discord.py model)",
        "version": sys.version.split()[0],
        "transport": "WebSocket (loopback via stdlib socket)",
        "events_processed": events_ingested,
        "duration_ms": burst_duration,
        "events_per_sec": eps,
        "p50_us": p50,
        "p95_us": p95,
        "p99_us": p99,
        "heap_used_mb": peak_mem / (1024 * 1024),
        "cached_messages": len(message_cache),
        "micro_snowflake_ns": micro_snowflake_ns,
        "micro_permission_ns": micro_perm_ns,
        "micro_route_ns": micro_route_ns
    }
    print(json.dumps(result, indent=2))
    sys.exit(0)

def run_standalone(opts):
    tracemalloc.start()
    frames = [generate_guild_create(opts.guild_members)]
    for i in range(1, opts.events):
        frames.append(generate_payload(i + 2, i))

    message_cache = {}
    role_map = {}
    latencies = []

    burst_start = time.perf_counter()

    for frame in frames:
        t0 = time.perf_counter()
        data = json.loads(frame)
        t = data.get("t")

        if t == "GUILD_CREATE":
            guild = data.get("d", {})
            for r in guild.get("roles", []):
                role_map[r["id"]] = r
        elif t == "MESSAGE_CREATE":
            msg = data.get("d", {})
            message_cache[msg["id"]] = msg
            parse_snowflake(msg["id"])
            member = msg.get("member")
            if member and "roles" in member:
                resolve_permissions(member["roles"], role_map)
            normalize_route("POST", f"/channels/{msg['channel_id']}/messages/{msg['id']}")

        elapsed_us = (time.perf_counter() - t0) * 1_000_000.0
        latencies.append(elapsed_us)

    burst_duration_ms = (time.perf_counter() - burst_start) * 1000.0

    current_mem, peak_mem = tracemalloc.get_traced_memory()
    tracemalloc.stop()

    latencies.sort()
    p50 = latencies[int(len(latencies) * 0.50)] if latencies else 0
    p95 = latencies[int(len(latencies) * 0.95)] if latencies else 0
    p99 = latencies[int(len(latencies) * 0.99)] if latencies else 0
    eps = (len(frames) / (burst_duration_ms / 1000.0)) if burst_duration_ms > 0 else 0

    # Micro-benchmarks
    MICRO_ITERS = 5000
    sample_snowflake = snowflake_id(12345)
    t_start = time.perf_counter()
    for _ in range(MICRO_ITERS):
        parse_snowflake(sample_snowflake)
    micro_snowflake_ns = ((time.perf_counter() - t_start) / MICRO_ITERS) * 1_000_000_000.0

    sample_roles = [snowflake_id(11), snowflake_id(12)]
    t_start = time.perf_counter()
    for _ in range(MICRO_ITERS):
        resolve_permissions(sample_roles, role_map)
    micro_perm_ns = ((time.perf_counter() - t_start) / MICRO_ITERS) * 1_000_000_000.0

    sample_route = f"/channels/{snowflake_id(123)}/messages/{snowflake_id(456)}"
    t_start = time.perf_counter()
    for _ in range(MICRO_ITERS):
        normalize_route("POST", sample_route)
    micro_route_ns = ((time.perf_counter() - t_start) / MICRO_ITERS) * 1_000_000_000.0

    sample_frame = frames[1]
    t_start = time.perf_counter()
    for _ in range(MICRO_ITERS):
        p = json.loads(sample_frame)
        _ = p.get("t") == "MESSAGE_CREATE"
    micro_filter_ns = ((time.perf_counter() - t_start) / MICRO_ITERS) * 1_000_000_000.0

    result = {
        "runtime": "Python (CPython / discord.py model)",
        "version": sys.version.split()[0],
        "transport": "In-memory dynamic pipeline",
        "events_processed": len(frames),
        "duration_ms": burst_duration_ms,
        "events_per_sec": eps,
        "p50_us": p50,
        "p95_us": p95,
        "p99_us": p99,
        "heap_used_mb": peak_mem / (1024 * 1024),
        "cached_messages": len(message_cache),
        "micro_snowflake_ns": micro_snowflake_ns,
        "micro_permission_ns": micro_perm_ns,
        "micro_route_ns": micro_route_ns,
        "micro_filter_ns": micro_filter_ns
    }
    print(json.dumps(result, indent=2))
    sys.exit(0)

def main():
    opts = parse_args()
    if opts.connect:
        run_websocket(opts)
    else:
        run_standalone(opts)

if __name__ == "__main__":
    main()
