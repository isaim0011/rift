# Rift MCP Server (Cloudflare Worker & Node.js)

The official **Model Context Protocol (MCP)** server for **Rift** — empowering AI coding agents (Claude Desktop, Cursor, Antigravity, GitHub Copilot) to generate, validate, and debug Discord bots using the Rift engine.

## Features
- **`search_docs`**: Search full Discord v10 REST API endpoints, types, and Rift crate docs.
- **`generate_bot`**: Instant production boilerplate in Rust, TypeScript, Bun, or Python.
- **`calculate_permissions`**: Bitwise calculator and decoder for Discord permissions.
- **`validate_intents`**: Gateway intent safety checker (identifies privileged intent requirements).

## 1-Click Cloudflare Workers Deployment

Deploy free to Cloudflare Workers with zero infrastructure:

```bash
# 1. Install wrangler if not already installed
npm install -g wrangler

# 2. Login to Cloudflare
wrangler login

# 3. Deploy
wrangler deploy
```

Your MCP server will be live at:
`https://rift-mcp.<your-subdomain>.workers.dev/mcp`

---

## Configuring with AI Agents

### 1. Claude Desktop (`claude_desktop_config.json`)
```json
{
  "mcpServers": {
    "rift": {
      "command": "npx",
      "args": ["-y", "mcp-proxy", "https://rift-mcp.workers.dev/mcp"]
    }
  }
}
```

### 2. Cursor (`.cursor/mcp.json`)
```json
{
  "mcpServers": {
    "rift": {
      "url": "https://rift-mcp.workers.dev/mcp"
    }
  }
}
```
