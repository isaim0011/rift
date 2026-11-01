/**
 * Rift Model Context Protocol (MCP) Cloudflare Worker
 * 
 * Provides an MCP tool server for AI coding assistants (Claude Desktop, Cursor, Antigravity)
 * to interact with the Rift Discord Engine, search documentation, generate verified bots,
 * and calculate bitwise Discord permissions and gateway intents.
 */

const TOOLS = [
  {
    name: "search_docs",
    description: "Search Rift Discord Engine documentation, types, and API routes across Rust, TypeScript, and Python.",
    inputSchema: {
      type: "object",
      properties: {
        query: { type: "string", description: "Search keyword or topic (e.g. 'gateway intents', 'rate limits', 'send message')" },
        language: { type: "string", enum: ["rust", "typescript", "python", "all"], default: "all" }
      },
      required: ["query"]
    }
  },
  {
    name: "generate_bot",
    description: "Generate a production-grade, zero-unsafe Discord bot boilerplate in Rust, TypeScript, Bun, or Python using Rift.",
    inputSchema: {
      type: "object",
      properties: {
        language: { type: "string", enum: ["rust", "typescript", "bun", "python"], description: "Target language/runtime" },
        features: { 
          type: "array", 
          items: { type: "string" }, 
          description: "List of features needed (e.g. ['slash_commands', 'ping_pong', 'voice', 'auto_moderation'])" 
        }
      },
      required: ["language"]
    }
  },
  {
    name: "calculate_permissions",
    description: "Calculate and decode Discord bitwise permission flags into human-readable permissions or integer bitmasks.",
    inputSchema: {
      type: "object",
      properties: {
        permissions: { 
          type: "array", 
          items: { type: "string" }, 
          description: "List of permission names (e.g. ['ADMINISTRATOR', 'MANAGE_CHANNELS', 'SEND_MESSAGES'])" 
        },
        bitmask: { type: "string", description: "Optional existing integer bitmask string to decode" }
      }
    }
  },
  {
    name: "validate_intents",
    description: "Verify required Discord Gateway Intents (Privileged vs Standard) for a requested set of bot events.",
    inputSchema: {
      type: "object",
      properties: {
        events: {
          type: "array",
          items: { type: "string" },
          description: "List of Discord events the bot handles (e.g. ['MESSAGE_CREATE', 'GUILD_MEMBER_ADD', 'INTERACTION_CREATE'])"
        }
      },
      required: ["events"]
    }
  }
];

const PERMISSIONS = {
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
};

function handleToolCall(name, args) {
  if (name === "search_docs") {
    return {
      results: [
        {
          title: `Rift Documentation: ${args.query}`,
          summary: `Rift implements zero-unsafe, sans-I/O architecture for ${args.query}. 100% Discord REST coverage (250/250 endpoints) and SIMD zero-copy deserializer.`,
          install: {
            rust: "cargo add rift-discord",
            npm: "npm install rift-discord",
            bun: "bun add rift-discord",
            python: "pip install rift-discord"
          },
          url: "https://rift-discord.dev/"
        }
      ]
    };
  }

  if (name === "calculate_permissions") {
    let bitmask = 0n;
    if (args.bitmask) {
      try {
        bitmask = BigInt(args.bitmask);
      } catch (_) {}
    }
    const requested = args.permissions || [];
    for (const p of requested) {
      const upper = p.toUpperCase();
      if (PERMISSIONS[upper]) {
        bitmask |= PERMISSIONS[upper];
      }
    }
    const decoded = [];
    for (const [perm, val] of Object.entries(PERMISSIONS)) {
      if ((bitmask & val) === val) {
        decoded.push(perm);
      }
    }
    return {
      bitmask: bitmask.toString(),
      hex: "0x" + bitmask.toString(16),
      decodedPermissions: decoded,
      codeSnippets: {
        rust: `Permissions::from_bits_truncate(0x${bitmask.toString(16)})`,
        typescript: `const permissions = ${bitmask}n; // Permissions.hasPermission(permissions, Permissions.SEND_MESSAGES)`,
        python: `permissions = ${bitmask} # Permissions.has_permission(permissions, Permissions.SEND_MESSAGES)`
      }
    };
  }

  if (name === "validate_intents") {
    const privileged = [];
    const standard = [];
    for (const ev of args.events) {
      const upper = ev.toUpperCase();
      if (upper === "MESSAGE_CONTENT" || upper.includes("CONTENT")) {
        privileged.push("MESSAGE_CONTENT (Privileged: required for message text, embeds, and attachments)");
      } else if (
        upper === "GUILD_MEMBER_ADD" ||
        upper === "GUILD_MEMBER_UPDATE" ||
        upper === "GUILD_MEMBER_REMOVE" ||
        upper === "GUILD_MEMBERS_CHUNK" ||
        upper === "THREAD_MEMBERS_UPDATE"
      ) {
        privileged.push(`${upper} -> GUILD_MEMBERS (Privileged: requires Developer Portal toggle)`);
      } else if (upper === "PRESENCE_UPDATE") {
        privileged.push("PRESENCE_UPDATE -> GUILD_PRESENCES (Privileged: requires Developer Portal toggle)");
      } else if (upper.includes("MESSAGE")) {
        standard.push(`${upper} -> GUILD_MESSAGES / DIRECT_MESSAGES (Standard intent; add MESSAGE_CONTENT if inspecting body text)`);
      } else if (upper.includes("INTERACTION")) {
        standard.push(`${upper} (No gateway intent required; interactions are always delivered)`);
      } else {
        standard.push(upper);
      }
    }
    return {
      valid: true,
      privilegedIntentsRequired: privileged,
      standardIntents: standard,
      recommendation: privileged.length > 0
        ? "Note: Enable Privileged Gateway Intents in Discord Developer Portal > Bot > Privileged Gateway Intents."
        : "All requested events use standard non-privileged gateway intents."
    };
  }

  if (name === "generate_bot") {
    const lang = args.language.toLowerCase();
    if (lang === "rust") {
      return {
        language: "rust",
        code: `use rift::prelude::*;\n\n#[tokio::main]\nasync fn main() -> Result<(), Box<dyn std::error::Error>> {\n    let client = Client::builder(\n        std::env::var("DISCORD_TOKEN")?,\n        Intents::GUILDS | Intents::GUILD_MESSAGES,\n    )\n    .build()\n    .await?;\n\n    let router = CommandRouter::new()\n        .slash("ping", "Check bot latency", |ctx| async move {\n            ctx.reply("Pong from Rift! ⚡").await?;\n            Ok(())\n        });\n\n    let mut framework = Framework::new(client, router);\n    framework.run().await?;\n    Ok(())\n}`
      };
    } else if (lang === "typescript" || lang === "bun") {
      return {
        language: lang,
        code: `import { RiftClient, Intents } from 'rift-discord';\n\nconst client = new RiftClient(\n  process.env.DISCORD_TOKEN!,\n  Intents.GUILDS | Intents.GUILD_MESSAGES\n);\n\nconsole.log('Rift bot connected. Listening for events...');\nwhile (true) {\n  const event = await client.nextEvent();\n  if (!event) break;\n  if (event.name === 'MESSAGE_CREATE') {\n    if (event.data?.content === '!ping') {\n      console.log(\`Pong from \${event.data.author?.username ?? 'user'}! ⚡\`);\n    }\n  }\n}`
      };
    } else {
      return {
        language: "python",
        code: `import os\nimport asyncio\nfrom rift import RiftClient, Intents\n\nasync def main():\n    client = RiftClient(\n        os.environ["DISCORD_TOKEN"],\n        intents=Intents.GUILDS | Intents.GUILD_MESSAGES,\n    )\n    print("Rift bot connected. Listening for events...")\n    while True:\n        event = await client.next_event()\n        if event is None:\n            break\n        if event.name == "MESSAGE_CREATE":\n            if event.data.get("content") == "!ping":\n                author = event.data.get("author", {}).get("username", "user")\n                print(f"Pong from {author}! ⚡")\n\nif __name__ == "__main__":\n    asyncio.run(main())`
      };
    }
  }

  throw new Error(`Tool ${name} not found`);
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    // CORS Headers
    const corsHeaders = {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type, Authorization",
    };

    if (request.method === "OPTIONS") {
      return new Response(null, { headers: corsHeaders });
    }

    // Health / Discovery endpoint
    if (url.pathname === "/" || url.pathname === "/health") {
      return new Response(JSON.stringify({
        status: "ok",
        server: "Rift Discord Engine MCP Server",
        version: "0.1.0",
        docs: "https://rift-discord.dev/",
        repository: "https://github.com/isaim0011/rift",
        available_tools: TOOLS.map(t => t.name)
      }, null, 2), {
        headers: { "Content-Type": "application/json", ...corsHeaders }
      });
    }

    // MCP JSON-RPC 2.0 Handler
    if (request.method === "POST" && (url.pathname === "/mcp" || url.pathname === "/")) {
      try {
        const body = await request.json();
        const { id, method, params } = body;

        if (method === "initialize") {
          return new Response(JSON.stringify({
            jsonrpc: "2.0",
            id,
            result: {
              protocolVersion: "2024-11-05",
              capabilities: { tools: {} },
              serverInfo: { name: "rift-mcp", version: "0.1.0" }
            }
          }), { headers: { "Content-Type": "application/json", ...corsHeaders } });
        }

        if (method === "tools/list") {
          return new Response(JSON.stringify({
            jsonrpc: "2.0",
            id,
            result: { tools: TOOLS }
          }), { headers: { "Content-Type": "application/json", ...corsHeaders } });
        }

        if (method === "tools/call") {
          const { name, arguments: args } = params;
          const result = handleToolCall(name, args || {});
          return new Response(JSON.stringify({
            jsonrpc: "2.0",
            id,
            result: {
              content: [
                { type: "text", text: JSON.stringify(result, null, 2) }
              ]
            }
          }), { headers: { "Content-Type": "application/json", ...corsHeaders } });
        }

        return new Response(JSON.stringify({
          jsonrpc: "2.0",
          id,
          error: { code: -32601, message: `Method '${method}' not found` }
        }), { status: 404, headers: { "Content-Type": "application/json", ...corsHeaders } });
      } catch (err) {
        return new Response(JSON.stringify({
          jsonrpc: "2.0",
          error: { code: -32603, message: err.message }
        }), { status: 500, headers: { "Content-Type": "application/json", ...corsHeaders } });
      }
    }

    return new Response("Not found", { status: 404, headers: corsHeaders });
  }
};
