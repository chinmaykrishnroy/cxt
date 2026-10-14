# Agent skills: CLI and TUI export

cxt 0.5.0 includes an instruction-only, portable [SKILL.md](../skills/cxt/SKILL.md). It teaches bounded task selection, representations, Git review, tracing, token accounting, safe exports, TUI, MCP, configuration, caching and evaluation. It does not install cxt, grant execution permissions or register an account.

## Export from a project

```bash
# Generic portable folder; no source scan or configuration loading
cxt --export-skill

# Pick your agent's project-local discovery location
cxt --export-skill --skill-agent codex
cxt --export-skill --skill-agent claude
cxt --export-skill --skill-agent antigravity
cxt --export-skill --skill-agent devin
cxt --export-skill --skill-agent kiro

# Exact destination wins over the convention; quote spaces
cxt --export-skill "my-guides/cxt/SKILL.md"
```

Run **one** appropriate command, not all of them. Existing files (including an older exported guide) are never overwritten. Review/remove or archive an old guide yourself, or export to a new filename for comparison. Parent directories are created; supply a file path, not a directory. Success reports the actual destination on stderr. Failure exits with code 3. Skill export is independent of source scanning, clipboard, MCP and context delivery; those modes cannot be combined.

| Agent | Project-local destination | Activation / remaining step |
| --- | --- | --- |
| Generic | `cxt-skill/SKILL.md` | Import/copy the folder using the client's supported mechanism. |
| Codex | `.agents/skills/cxt/SKILL.md` | Automatic matching or `$cxt`; restart if changes are not discovered. |
| Claude Code | `.claude/skills/cxt/SKILL.md` | Automatic matching or `/cxt`. |
| Antigravity | `.agents/skills/cxt/SKILL.md` | Project skill discovery; reload the client if needed. |
| Devin | `.agents/skills/cxt/SKILL.md` | Commit to the repository connected to Devin; reference `@skills:cxt`. Export does not commit or push. |
| Kiro | `.kiro/skills/cxt/SKILL.md` | Project discovery / `/cxt`; custom agents may need a skill resource entry. |

These paths follow the official documentation checked on 2026-10-14. Filesystem export is tested; activation in every hosted/desktop agent is not independently verified. Skills and MCP are separate integrations: a skill gives instructions, while MCP exposes constrained context tools. Neither makes cxt a fully resolved semantic code search engine.

## Export from the TUI

```bash
cxt --tui --skill-agent claude
```

Press **E** (Shift+e), inspect/change the filename and press Enter. Set **Skill export agent** in Settings (`6` / F2) to change the next default. The dialog stays open on errors so you can choose a new path. Exporting the static guide does not export source, bypass a failed context scan, or exit the TUI. `g` remains the reviewed **context** export command.

## Personal / global installation

Project-local export is the default: no administrator access or global configuration edits. To install personally, explicitly choose a user location supported by your client:

```powershell
# Codex personal skills (PowerShell)
cxt --export-skill "$HOME/.agents/skills/cxt/SKILL.md"
```

```bash
# Claude Code personal skills (bash/zsh)
cxt --export-skill "$HOME/.claude/skills/cxt/SKILL.md"
```

Codex uses `~/.agents/skills`; Claude Code uses `~/.claude/skills`; Kiro uses `~/.kiro/skills`. Antigravity's app/IDE uses `~/.gemini/config/skills`, whereas its CLI uses `~/.gemini/antigravity-cli/skills`. Pass the full `.../cxt/SKILL.md` destination explicitly. Devin cloud skills belong in its connected repository or supported plugin system, not an invented local home directory. Do not assume personal skills synchronize to unrelated cloud products.

Kiro also supports importing a local skill folder or this repository's [`skills/cxt` folder](https://github.com/chinmaykrishnroy/cxt/tree/main/skills/cxt). For a custom Kiro agent, review its resource configuration; a documented pattern is `skill://.kiro/skills/*/SKILL.md`. cxt deliberately does not rewrite client JSON/TOML, enable tools, or submit marketplace entries.

## Sources and maintenance

- [Codex skills](https://developers.openai.com/codex/skills/)
- [Claude Code skills](https://code.claude.com/docs/en/skills)
- [Antigravity skills](https://antigravity.google/docs/skills)
- [Devin skills](https://docs.devin.ai/product-guides/skills)
- [Kiro skills](https://kiro.dev/docs/skills/)

The bundled guide is versioned with the executable. After upgrading, export it to a new path, review changes and replace the installed copy deliberately. An agent must still inspect the output, omissions and warnings; no skill can guarantee that static retrieval found all code needed for a task.
