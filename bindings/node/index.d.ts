/**
 * High-performance Node/Bun bindings for Rift: Discord Engine written in Rust.
 */

export interface SnowflakeParts {
  timestampMs: number;
  workerId: number;
  processId: number;
  increment: number;
}

export namespace Snowflake {
  export function timestampMs(id: string | bigint | number): number;
  export function workerId(id: string | bigint | number): number;
  export function processId(id: string | bigint | number): number;
  export function increment(id: string | bigint | number): number;
  export function fromParts(
    timestampMs: number,
    workerId: number,
    processId: number,
    increment: number
  ): string;
  export function deconstruct(id: string | bigint | number): SnowflakeParts;
}

export interface Overwrite {
  id: string;
  type: 0 | 1; // 0 = Role, 1 = Member
  allow: string | bigint;
  deny: string | bigint;
}

export namespace Permissions {
  export const CREATE_INSTANT_INVITE: bigint;
  export const KICK_MEMBERS: bigint;
  export const BAN_MEMBERS: bigint;
  export const ADMINISTRATOR: bigint;
  export const MANAGE_CHANNELS: bigint;
  export const MANAGE_GUILD: bigint;
  export const ADD_REACTIONS: bigint;
  export const VIEW_AUDIT_LOG: bigint;
  export const PRIORITY_SPEAKER: bigint;
  export const STREAM: bigint;
  export const VIEW_CHANNEL: bigint;
  export const SEND_MESSAGES: bigint;
  export const SEND_TTS_MESSAGES: bigint;
  export const MANAGE_MESSAGES: bigint;
  export const EMBED_LINKS: bigint;
  export const ATTACH_FILES: bigint;
  export const READ_MESSAGE_HISTORY: bigint;
  export const MENTION_EVERYONE: bigint;
  export const USE_EXTERNAL_EMOJIS: bigint;
  export const VIEW_GUILD_INSIGHTS: bigint;
  export const CONNECT: bigint;
  export const SPEAK: bigint;
  export const MUTE_MEMBERS: bigint;
  export const DEAFEN_MEMBERS: bigint;
  export const MOVE_MEMBERS: bigint;
  export const USE_VAD: bigint;
  export const CHANGE_NICKNAME: bigint;
  export const MANAGE_NICKNAMES: bigint;
  export const MANAGE_ROLES: bigint;
  export const MANAGE_WEBHOOKS: bigint;
  export const MANAGE_GUILD_EXPRESSIONS: bigint;
  export const USE_APPLICATION_COMMANDS: bigint;
  export const REQUEST_TO_SPEAK: bigint;
  export const MANAGE_EVENTS: bigint;
  export const MANAGE_THREADS: bigint;
  export const CREATE_PUBLIC_THREADS: bigint;
  export const CREATE_PRIVATE_THREADS: bigint;
  export const USE_EXTERNAL_STICKERS: bigint;
  export const SEND_MESSAGES_IN_THREADS: bigint;
  export const USE_EMBEDDED_ACTIVITIES: bigint;
  export const MODERATE_MEMBERS: bigint;
  export const VIEW_CREATOR_MONETIZATION_ANALYTICS: bigint;
  export const USE_SOUNDBOARD: bigint;
  export const CREATE_GUILD_EXPRESSIONS: bigint;
  export const CREATE_EVENTS: bigint;
  export const USE_EXTERNAL_SOUNDS: bigint;
  export const SEND_VOICE_MESSAGES: bigint;
  export const SET_VOICE_CHANNEL_STATUS: bigint;
  export const SEND_POLLS: bigint;
  export const USE_EXTERNAL_APPS: bigint;
  export const PIN_MESSAGES: bigint;
  export const BYPASS_SLOWMODE: bigint;
  export const ALL: bigint;

  export function hasPermission(
    permissions: string | bigint | number,
    required: string | bigint | number
  ): boolean;

  export function computeChannelPermissions(
    basePermissions: string | bigint | number,
    overwrites: Overwrite[],
    guildId: string | bigint | number,
    memberId: string | bigint | number,
    memberRoles: (string | bigint | number)[]
  ): string;
}

export namespace Intents {
  export const GUILDS: number;
  export const GUILD_MEMBERS: number;
  export const GUILD_MODERATION: number;
  export const GUILD_EXPRESSIONS: number;
  export const GUILD_INTEGRATIONS: number;
  export const GUILD_WEBHOOKS: number;
  export const GUILD_INVITES: number;
  export const GUILD_VOICE_STATES: number;
  export const GUILD_PRESENCES: number;
  export const GUILD_MESSAGES: number;
  export const GUILD_MESSAGE_REACTIONS: number;
  export const GUILD_MESSAGE_TYPING: number;
  export const DIRECT_MESSAGES: number;
  export const DIRECT_MESSAGE_REACTIONS: number;
  export const DIRECT_MESSAGE_TYPING: number;
  export const MESSAGE_CONTENT: number;
  export const GUILD_SCHEDULED_EVENTS: number;
  export const AUTO_MODERATION_CONFIGURATION: number;
  export const AUTO_MODERATION_EXECUTION: number;
  export const GUILD_MESSAGE_POLLS: number;
  export const DIRECT_MESSAGE_POLLS: number;
  export const PRIVILEGED: number;
  export const ALL_KNOWN: number;
}

export namespace Capabilities {
  export const NONE: number;
  export const CHANNEL_OBFUSCATION: number;
  export const IS_COMPONENTS_V2: number;
}

export function verifyInteraction(
  publicKeyHex: string,
  signatureHex: string,
  timestamp: string,
  body: Uint8Array | Buffer | string
): boolean;

export interface NormalizedRoute {
  id: string;
  template: string;
  major: string;
}

export function normalizeRoute(method: string, path: string): NormalizedRoute;

export interface AcquireVerdict {
  verdict: "go" | "wait" | "blocked";
  retryAfterMs?: number;
  reason?: string;
}

export interface ResponseHeaders {
  limit?: number;
  remaining?: number;
  resetAfterMs?: number;
  bucket?: string;
  global?: boolean;
  scope?: "user" | "global" | "shared" | string;
  retryAfterMs?: number;
}

export class RateLimiter {
  constructor(globalPerSecond?: number, invalidLimit?: number, invalidTripAt?: number);
  acquire(method: string, path: string, nowMs?: number): AcquireVerdict;
  onResponse(
    method: string,
    path: string,
    status: number,
    headers: ResponseHeaders,
    nowMs?: number
  ): void;
  invalidRequests(nowMs?: number): number;
}

export interface CacheConfig {
  maxMessagesPerChannel?: number;
  maxMembersPerGuild?: number;
  maxUsers?: number;
}

export interface CacheStats {
  guilds: number;
  visibleChannels: number;
  obfuscatedChannels: number;
  roles: number;
  members: number;
  users: number;
  messages: number;
}

export class Cache {
  constructor(config?: CacheConfig);
  update(eventOrEnvelope: string | object): void;
  stats(): CacheStats;
  isChannelObfuscated(channelId: string | bigint | number): boolean;
  getGuild(guildId: string | bigint | number): any | null;
  getChannel(channelId: string | bigint | number): any | null;
  getMember(guildId: string | bigint | number, userId: string | bigint | number): any | null;
  getMessage(channelId: string | bigint | number, messageId: string | bigint | number): any | null;
  computeMemberPermissions(
    guildId: string | bigint | number,
    channelId: string | bigint | number,
    userId: string | bigint | number
  ): string;
  clear(): void;
}

export interface HeartbeatPollResult {
  action: "wait" | "send" | "zombie";
  seq?: number;
}

export interface ReconnectPlan {
  plan: "resume" | "identify" | "fatal";
  url?: string;
  sessionId?: string;
  seq?: number;
  closeCode?: number;
}

export class GatewaySession {
  constructor();
  onHello(nowMs: number, intervalMs: number, jitter?: number): void;
  onReady(sessionId: string, resumeGatewayUrl: string): void;
  recordSeq(seq: number): void;
  seq(): number | null;
  isEstablished(): boolean;
  pollHeartbeat(nowMs: number): HeartbeatPollResult;
  planReconnect(
    why: "close" | "reconnect_requested" | "invalid_session" | "zombie",
    closeCode?: number,
    resumable?: boolean
  ): ReconnectPlan;
  reset(): void;
}

export interface ClientEvent {
  shardId: number;
  name: string;
  data: any;
}

export class RiftClient {
  constructor(token: string, intents?: number, capabilities?: number);
  nextEvent(): Promise<ClientEvent | null>;
  sendCommand(shardId: number, opcode: number, payload: any): void;
  updatePresence(
    shardId: number,
    status: "online" | "dnd" | "idle" | "invisible",
    activityName?: string,
    activityType?: number
  ): void;
  stop(): Promise<void>;
  injectMockEvent(shardId: number, name: string, payload: any): void;
}
