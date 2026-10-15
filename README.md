# oclikits

English | [简体中文](README.zh-CN.md)

A lightweight command-line toolkit built on [OpenCode](https://opencode.ai). It wraps OpenCode's agents, models, tools and sessions into a few short everyday terminal commands.

`oclikits` is the renamed and rewritten successor of `aiclikits`:

- It does not implement its own LLM client, and does not manage API keys or base URLs.
- It does not store chat history itself. Messages, tool calls and context compaction are all handled by OpenCode.
- Models are not hard-coded. A global default and per-command models are set in `~/.config/oclikits/config.jsonc`.
- It only maintains a mapping from logical context to OpenCode `sessionID`.
- Tasks run through `opencode run`, and `oclikits` manages the session lifecycle.

## Requirements

- [OpenCode v2](https://opencode.ai), with providers and models already configured (login/API keys).
- Rust toolchain (only needed for building).

## Installation

```bash
./install.sh
```

The installer builds the release binary and creates the main command `oc` plus short aliases in `~/.local/bin`:

```text
ocq ocsq oct ocsh occmt octrans
aiq aiask aish ait aitrans   # legacy aliases, kept for compatibility
```

The installer also creates a model config template at `~/.config/oclikits/config.jsonc` (an existing file is never overwritten). Fill in its `model` field after installation. See the [Models](#models) section below.

Make sure `~/.local/bin` is in your `PATH`.

## Commands

| Command | Description | Session |
| :--- | :--- | :--- |
| `ocq` | Continuous quick Q&A | One global persistent session |
| `ocsq` | Single question, no history | Ephemeral session, deleted after the run |
| `oct` | Chinese–English translation and typo correction | Ephemeral session, deleted after the run |
| `ocsh` | Shell agent bound to the current directory | One persistent session per directory |
| `occmt` | Git commits (Conventional Commits) | One ephemeral session per repository |

Subcommands are also available through the main command: `oc q`, `oc sq`, `oc trans`, `oc shell`, `oc commit`.

### ocq

```bash
ocq "What is the difference between a Swift actor and a class?"
ocq "What about the performance overhead of actors?"
ocq "Compare that with the class version"
```

- Consecutive calls belong to the same OpenCode session, so context is preserved.
- Sessions are not separated by directory, and there is no turn limit. Context compaction is handled automatically by OpenCode.
- `-n, --new`: delete the old session and start a new one.
- `--clear`: delete the session and exit.

### ocsq

```bash
ocsq "What is the difference between HTTP 409 and 422?"
```

A single question and a single answer. It does not pollute the `ocq` session history, so it suits small questions that don't need follow-up.

### oct

```bash
oct "apple"
oct "今天天气不错"
oct "teh quick brown fox"
```

- Detects whether the input is Chinese or English and translates accordingly.
- A single word returns several common, high-quality translations; a sentence returns one best translation.
- English spelling and grammar errors are corrected before translation.
- Each translation is an independent session with no shared history.

### ocsh

```bash
cd ~/project
ocsh "Find out why the startup fails"
ocsh "Was the error I just saw caused by the configuration?"

cd /tmp
ocsh "Show what takes up the most disk space"
```

- The current working directory (after canonicalization) maps to exactly one OpenCode session.
- Repeated calls in the same directory keep context; different directories are fully isolated.
- `-n, --new`: clear the current directory's session and run the task in a new one.
- `--clear`: clear only the current directory's session, leaving other directories untouched.
- `--session` / `--list` / `--switch` are not provided. For complex session management, use the OpenCode TUI or Web UI.

### occmt

```bash
occmt
occmt --zh
occmt --en
occmt -m opencode-go/mimo-v2.6-flash
```

- Works on the current directory: if inside a Git repository, only that repository is processed; otherwise it searches up to two levels of subdirectories.
- If several repositories with changes are found, they are listed and confirmed first.
- Commit messages follow Conventional Commits. The default language is determined by project instructions (such as AGENTS.md) and repository context.
- `--en` forces English; `--zh` forces Simplified Chinese (the type prefix remains in English).
- Only commits. It never runs `git push`, never amends, and never creates empty commits.
- Each repository uses its own temporary session, which is deleted automatically when the task ends (including on failure or interruption).

## Output behavior

- While running, text returned by OpenCode is shown in light gray as a live preview (streaming).
- When the final result is ready, the preview and intermediate output are cleared, leaving only the final result.
- When output is redirected or piped (stdout is not a terminal), no preview is shown and only the final result is printed:

```bash
ocq "Explain what mmap is" > answer.txt
ocsq "What does HTTP status 418 mean?" | pbcopy
```

- Use `--debug` to see the raw JSON events and the full intermediate output (written to stderr).

## Models

Models are configured in a file rather than in the code:

```text
~/.config/oclikits/config.jsonc
```

```jsonc
{
  // Global default model: used by all commands that have no dedicated model (required).
  "model": "opencode-go/mimo-v2.6-flash",

  // Per-command models (optional). Leave empty to fall back to "model" above.
  "commands": {
    "q": "",      // ocq: continuous quick Q&A
    "sq": "",     // ocsq: single question (no history)
    "trans": "",  // oct / octrans: Chinese–English translation and typo correction
    "shell": "",  // ocsh: shell agent bound to the current directory
    "commit": ""  // occmt: Git commits
  }
}
```

- The `.jsonc` extension tells editors and tools to treat the file as "JSON with Comments", so comments won't cause errors.
- `install.sh` creates the commented template above. If the file is missing at runtime, it is generated automatically; existing files are never overwritten.
- When upgrading from the old `config.json`, if no `config.jsonc` exists, the old file is automatically renamed and migrated, so no settings are lost.
- The keys under `commands` are the canonical command names: `q`, `sq`, `trans`, `shell`, `commit`.
- All commands accept `-m, --model` to override the model temporarily, using OpenCode's `provider/model` format:

```bash
ocq -m opencode-go/deepseek-v4.1-flash "..."
```

- Resolution priority: `-m/--model` > `commands.<command>` > `model` > error prompting you to fix the config.

## Session storage

The session state file is created automatically. It lives in the same directory as `config.jsonc` and is independent of it:

```text
~/.config/oclikits/sessions.json
```

```json
{
  "version": 1,
  "ocq": { "session_id": "ses_xxx" },
  "ocsh": {
    "/Users/me/project": { "session_id": "ses_aaa" },
    "/tmp": { "session_id": "ses_bbb" }
  }
}
```

- The file is written via a temporary file followed by an atomic rename, and a file lock prevents concurrent overwrites.
- If the OpenCode session referenced locally no longer exists, the command clears the stale mapping, creates a new session, and retries once.
- Removing the local mapping and deleting the OpenCode session are two separate actions. A normal clear deletes the OpenCode session first, then removes the local record.

## Permissions and security

`ocsh` and `occmt` do not implement their own shell permission system. They rely entirely on OpenCode's `permission` configuration. Keep in mind:

- `opencode run` is non-interactive. Any permission that would require confirmation (`ask`) is **automatically rejected** (auto-reject).
- Operations you want to allow must be set to `allow` in the OpenCode configuration; operations to forbid should be set to `deny`.
- Whether read-only investigation commands (`ls`, `grep`, `git status`, etc.) run automatically is also decided by this configuration.

## Debugging

All commands support `--debug`, which prints the actual `opencode run` arguments and the raw JSON events (written to stderr):

```bash
ocsh --debug "Check disk usage"
```

## Legacy command compatibility

| Legacy command | New command |
| :--- | :--- |
| `aiq` / `aiask` | `ocq` |
| `aish` | `ocsh` |
| `ait` / `aitrans` | `oct` |

Legacy command names still work, but they show a one-time notice about the rename. The old `~/.config/aiclikits/config.json` (with `openai`, `models`, `defaults`, etc.) is no longer used. Providers and permissions are managed by OpenCode, and `oclikits` only stores its model settings in `~/.config/oclikits/config.jsonc`.

## Development

```bash
cargo build           # build
cargo test            # unit tests
cargo build --release # release build
```

Code layout:

```text
src/
├── main.rs             # Entry point: dispatch by argv[0] alias and subcommand
├── cli.rs              # clap command and argument definitions
├── config.rs           # config.jsonc loading and model resolution
├── error.rs            # Unified error type
├── i18n.rs             # Chinese/English messages
├── opencode/
│   ├── runner.rs       # Unified opencode run invocation and event output
│   ├── event.rs        # NDJSON event parsing
│   └── session.rs      # sessions.json read/write and OpenCode session deletion
├── commands/
│   ├── q.rs            # ocq
│   ├── sq.rs           # ocsq
│   ├── trans.rs        # oct
│   ├── shell.rs        # ocsh
│   └── commit.rs       # occmt
└── ui/
    └── mod.rs          # Terminal output such as elapsed time and confirmations
```

`docs/reference/occmt.sh` is the shell reference implementation of `occmt` from before the rewrite.
