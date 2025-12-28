<h1 align="center">Tacodex CLI</h1>
<p align="center">Multi-agent orchestration CLI based on OpenAI Codex</p>

<p align="center"><code>npm i -g tacodex</code></p>

> [!NOTE]
> This is the npm package distribution folder. The main Rust implementation is in `codex-rs/tui2`.

---

## Quickstart

Install globally:

```shell
npm install -g tacodex
```

Run interactively:

```shell
tacodex
```

Or, run with a prompt as input:

```shell
tacodex "explain this codebase to me"
```

---

## Configuration

Tacodex configuration files are stored in `~/.tacodex/`:

- `~/.tacodex/config.toml` - Main configuration
- `~/.tacodex/AGENTS.md` - Custom agent instructions

### Environment Variable

Set `TACODEX_HOME` to override the default config directory.

---

## Multi-Agent Orchestration

Tacodex supports multi-agent orchestration via `tacodex.toml`:

```toml
[orchestrator]
mode = "parallel"
max_agents = 4

[[agents]]
name = "researcher"
role = "Research and gather information"

[[agents]]
name = "implementer"
role = "Implement code changes"
```

---

## License

This repository is licensed under the [Apache-2.0 License](LICENSE).
