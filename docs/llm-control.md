# Letting an LLM control the M811: plan

## Is there an industry standard?

No single standard for "control a mouse's settings from an LLM". The pieces people use:

| Thing | What it is | Fit for us |
|---|---|---|
| CLI + text config | Any agent that can run a shell can use it | **Done now.** Simplest, works with every LLM tool |
| `AGENTS.md` | Common file name agents read for repo instructions (Codex, Claude Code via CLAUDE.md, Cursor, others) | **Done now** |
| MCP (Model Context Protocol) | The de facto standard for giving LLM apps tools. Claude, ChatGPT, Cursor, VS Code support it | **Next.** Thin wrapper over the CLI |
| OpenRGB SDK | Network protocol (TCP 6742) for RGB lighting across vendors | Later, lighting only. Would let OpenRGB and its plugins drive the mouse LEDs |
| libratbag / ratbagd | Linux D-Bus service for gaming mouse settings (Piper uses it) | Only if we go Linux |

## Phase 1 (this PR)

- `m811` CLI: `status`, `apply`, `check`, `unstick`, plus one command per setting
- Plain text config, same words as the CLI. `status` output is a valid config
- `AGENTS.md` with rules (never touch macro slot 0, batch tests in 3s)

## Phase 2: MCP server (dropped)

David prefers a paste-in doc over MCP: `m811 prompt` prints docs/LLM-PROMPT.md plus
current settings. Notes kept below in case it comes back.


- `m811 mcp` subcommand speaking MCP over stdio (JSON-RPC lines). No new crate needed for
  the stdio transport; `serde_json` is the one dep worth adding
- Tools: `get_status`, `set_lighting`, `set_dpi`, `set_poll_rate`, `set_side_key`,
  `write_macro`, `switch_profile`, `apply_config(text)`
- Each tool calls the same `cli::run`, so the slot 0 guard applies
- Register in Claude Desktop / Claude Code with one config entry pointing at `m811.exe mcp`

## Phase 3: hooks

- Done: `m811 tray` switches per game from games.cfg
- Port the side-key color ladder (`tools/m811_ladder.py`) into the app

## Open questions

- Map profiles 2-5 for DPI, poll and keys (blocks at 0x0100, 0x01B0, 0x0260, 0x0310 are a guess)
- Capture brightness and speed sliders from the vendor app
