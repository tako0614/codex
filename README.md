<p align="center"><code>npm i -g tacodex</code></p>

<p align="center"><strong>Tacodex</strong> is a multi-agent orchestration CLI based on OpenAI Codex.
</br>
</br>Fork of <a href="https://github.com/openai/codex">OpenAI Codex CLI</a> with multi-agent support.</p>

<p align="center">
  <img src="./.github/codex-cli-splash.png" alt="Tacodex CLI splash" width="80%" />
  </p>

---

## Quickstart

### Installing and running Tacodex

Install globally with npm:

```shell
npm install -g tacodex
```

Then simply run `tacodex` to get started:

```shell
tacodex
```

<details>
<summary>You can also go to the <a href="https://github.com/tako0614/codex/releases/latest">latest GitHub Release</a> and download the appropriate binary for your platform.</summary>

Each GitHub Release contains many executables, but in practice, you likely want one of these:

- macOS
  - Apple Silicon/arm64: `tacodex-aarch64-apple-darwin.tar.gz`
  - x86_64 (older Mac hardware): `tacodex-x86_64-apple-darwin.tar.gz`
- Linux
  - x86_64: `tacodex-x86_64-unknown-linux-musl.tar.gz`
  - arm64: `tacodex-aarch64-unknown-linux-musl.tar.gz`
- Windows
  - x86_64: `tacodex-x86_64-pc-windows-msvc.zip`

</details>

### Using Tacodex with your ChatGPT plan

<p align="center">
  <img src="./.github/codex-cli-login.png" alt="Tacodex CLI login" width="80%" />
  </p>

Run `tacodex` and select **Sign in with ChatGPT**. We recommend signing into your ChatGPT account to use Tacodex as part of your Plus, Pro, Team, Edu, or Enterprise plan.

You can also use Tacodex with an API key, but this requires [additional setup](./docs/authentication.md#usage-based-billing-alternative-use-an-openai-api-key).

### Model Context Protocol (MCP)

Tacodex can access MCP servers. To configure them, refer to the [config docs](./docs/config.md#mcp_servers).

### Configuration

Tacodex supports a rich set of configuration options, with preferences stored in `~/.tacodex/config.toml`. For full configuration options, see [Configuration](./docs/config.md).

### Multi-Agent Orchestration

Tacodex uses a parent-child agent architecture that runs entirely within the TUI:

1. **Parent Agent (Orchestrator)**: Plans, creates `.tacodex/` specifications, then spawns child agents
2. **Child Agents**: Execute actual coding based on parent's instructions

Everything completes in a single TUI session - no additional commands needed.

The parent agent automatically:
- Creates `.tacodex/SPEC.md`, `PLAN.md`, and `instructions/*.md`
- Spawns child agents (architect → backend → frontend → testing)
- Coordinates execution based on dependencies

### Docs & FAQ

- [**Getting started**](./docs/getting-started.md)
  - [CLI usage](./docs/getting-started.md#cli-usage)
  - [Slash Commands](./docs/slash_commands.md)
  - [Running with a prompt as input](./docs/getting-started.md#running-with-a-prompt-as-input)
  - [Example prompts](./docs/getting-started.md#example-prompts)
  - [Custom prompts](./docs/prompts.md)
  - [Memory with AGENTS.md](./docs/getting-started.md#memory-with-agentsmd)
- [**Configuration**](./docs/config.md)
  - [Example config](./docs/example-config.md)
- [**Sandbox & approvals**](./docs/sandbox.md)
- [**Execpolicy quickstart**](./docs/execpolicy.md)
- [**Authentication**](./docs/authentication.md)
  - [Auth methods](./docs/authentication.md#forcing-a-specific-auth-method-advanced)
  - [Login on a "Headless" machine](./docs/authentication.md#connecting-on-a-headless-machine)
- **Automating Tacodex**
  - [TypeScript SDK](./sdk/typescript/README.md)
  - [Non-interactive mode (`tacodex exec`)](./docs/exec.md)
- [**Advanced**](./docs/advanced.md)
  - [Tracing / verbose logging](./docs/advanced.md#tracing--verbose-logging)
  - [Model Context Protocol (MCP)](./docs/advanced.md#model-context-protocol-mcp)
- [**Contributing**](./docs/contributing.md)
- [**Install & build**](./docs/install.md)
  - [System Requirements](./docs/install.md#system-requirements)
  - [Build from source](./docs/install.md#build-from-source)
- [**FAQ**](./docs/faq.md)

---

## License

This repository is licensed under the [Apache-2.0 License](LICENSE).
