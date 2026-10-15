# oclikits

[English](README.md) | 简体中文

基于 [OpenCode](https://opencode.ai) 的轻量命令行工具集，把 OpenCode 的 Agent、模型、工具和 Session 能力封装成几个日常终端短命令。

原 `aiclikits` 已更名并重构为 `oclikits`：

- 不再实现自己的 LLM Client，不管理 API Key / Base URL；
- 不自己保存聊天历史，消息、工具调用与上下文压缩全部由 OpenCode 负责；
- 模型不再写死在代码里，可选通过 `~/.config/oclikits/config.jsonc` 配置全局默认值与各命令的专用模型；不配置时复用 OpenCode 自身的默认模型；
- 只维护“逻辑上下文 → OpenCode sessionID”的映射；
- 通过 `opencode run` 执行任务，并管理 session 生命周期。

## 依赖

- [OpenCode v2](https://opencode.ai)（需已完成 provider / 模型登录配置）
- Rust 工具链（仅构建时需要）

## 安装

```bash
./install.sh
```

安装后会在 `~/.local/bin` 生成主命令 `oc` 及短命令别名：

```text
ocq ocsq oct ocsh occmt octrans
aiq aiask aish ait aitrans   # 旧命令兼容别名
```

安装时会创建模型配置模板 `~/.config/oclikits/config.jsonc`（已存在则不覆盖），可选填写其中的 `model`；留空则复用 OpenCode 自身的默认模型。详见下方[模型](#模型)一节。

请确保 `~/.local/bin` 在 `PATH` 中。

## 命令

| 命令 | 说明 | Session |
| :--- | :--- | :--- |
| `ocq` | 连续快速问答 | 全局唯一持久会话 |
| `ocsq` | 单次问答，无历史 | 临时会话，执行后删除 |
| `oct` | 中英翻译、错别字纠正 | 临时会话，执行后删除 |
| `ocsh` | 绑定当前目录的 Shell Agent | 每个目录一个持久会话 |
| `occmt` | Git 提交（Conventional Commits） | 每个仓库一个临时会话 |

也可以通过主命令调用子命令：`oc q`、`oc sq`、`oc trans`、`oc shell`、`oc commit`。

### ocq

```bash
ocq "Swift actor 和 class 有什么区别？"
ocq "那 actor 的性能开销呢？"
ocq "和刚才的 class 对比一下"
```

- 三次调用属于同一个 OpenCode session，保持上下文连续；
- 不按目录区分，也不限制轮数，上下文压缩由 OpenCode 自动完成（compaction）；
- `-n, --new`：删除旧会话并开始新会话；
- `--clear`：删除会话并退出。

### ocsq

```bash
ocsq "HTTP 409 和 422 有什么区别？"
```

一次调用、一次回答，不污染 `ocq` 的会话历史，适合不值得继续讨论的小问题。

### oct

```bash
oct "apple"
oct "今天天气不错"
oct "teh quick brown fox"
```

- 自动判断中英文并翻译；
- 单词返回多个高质量常用译法，句子返回唯一最佳翻译；
- 英文拼写/语法错误会先纠正再翻译；
- 每次翻译都是独立会话，不共享历史。

### ocsh

```bash
cd ~/project
ocsh "看看为什么启动失败"
ocsh "刚才那个错误是不是配置导致的？"

cd /tmp
ocsh "看看哪些东西占空间"
```

- 当前工作目录（canonicalize 后）对应唯一一个 OpenCode session；
- 同一目录下多次调用保持上下文，不同目录完全隔离；
- 执行过程中会显示实际执行的 Shell 命令（`$` 前缀）以及其他工具调用，方便确认 Agent 做了什么；
- `-n, --new`：清除当前目录会话后执行新任务；
- `--clear`：只清除当前目录的会话，不影响其他目录；
- 不提供 `--session` / `--list` / `--switch`，需要复杂会话管理请直接使用 OpenCode TUI/Web。

### occmt

```bash
occmt
occmt --zh
occmt --en
occmt -m opencode-go/mimo-v2.6-flash
```

- 作用于当前目录：若在 Git 仓库内只处理该仓库；否则搜索最多两级子目录；
- 发现多个存在修改的仓库时先列出并确认；
- 提交信息遵循 Conventional Commits；默认语言由项目指令（如 AGENTS.md）与仓库上下文决定；
- `--en` 强制英文，`--zh` 强制简体中文（类型前缀仍用英文）；
- 只提交，不执行 `git push`，不 amend，不创建空提交；
- 每个仓库使用独立临时 session，任务结束（含失败与中断）后自动删除。

## 输出行为

- 运行过程中，OpenCode 返回的文本会以淡灰色实时显示（流式预览）；
- `ocsh` 会额外显示 Agent 实际执行的 Shell 命令（`$` 前缀）与其他工具调用；这些内容写入 stderr，即使 stdout 被重定向或管道化也仍然可见，并且不会被清除；
- 最终结果生成后，会清除之前的预览与中间过程内容，只保留最终结果（以及上面的命令记录）；
- 输出被重定向或管道化（stdout 不是终端）时不做预览，只输出最终结果：

```bash
ocq "解释一下什么是 mmap" > answer.txt
ocsq "HTTP 状态码 418 是什么" | pbcopy
```

- 使用 `--debug` 可以查看原始 JSON 事件与完整的中间过程（输出到 stderr）。

## 模型

模型不再写死在代码里，而是通过配置文件设置，路径为：

```text
~/.config/oclikits/config.jsonc
```

```jsonc
{
  // 全局默认模型：所有命令在未配置专用模型时使用它（可选）。
  // 留空或不配置时，不向 opencode 传 --model，改用 OpenCode 自身的默认模型。
  "model": "opencode-go/mimo-v2.6-flash",

  // 各命令的专用模型（可选），留空则回退到上面的 "model"。
  "commands": {
    "q": "",      // ocq：连续快速问答
    "sq": "",     // ocsq：单次问答（无历史）
    "trans": "",  // oct / octrans：中英翻译、错别字纠正
    "shell": "",  // ocsh：绑定当前目录的 Shell Agent
    "commit": ""  // occmt：Git 提交
  }
}
```

- 使用 `.jsonc` 扩展名，编辑器/工具会按「JSON with Comments」处理，不会因为注释报错；
- 安装时（`install.sh`）会自动创建上面这份带注释的模板；若文件缺失，运行时也会兜底生成，已存在的文件不会被覆盖；
- 从旧版 `config.json` 升级时，如果存在旧文件且没有 `config.jsonc`，会自动重命名迁移，不会丢配置；
- `commands` 的键为命令的规范名：`q`、`sq`、`trans`、`shell`、`commit`；
- 所有命令都支持 `-m, --model`，用于临时覆盖，格式为 OpenCode 的 `provider/model`：

```bash
ocq -m opencode-go/deepseek-v4.1-flash "..."
```

- 解析优先级：`-m/--model` > `commands.<命令>` > `model` > OpenCode 自身的默认模型。也就是说，oclikits 未配置任何模型时，不再报错，而是不传 `--model`，由 OpenCode 按自身优先级（全局/项目配置、会话已选模型）决定。

## 会话存储

会话状态文件（自动创建，与 `config.jsonc` 同目录、相互独立）：

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

- 文件采用“写入临时文件 + 原子 rename”，并使用文件锁避免并发覆盖；
- 如果本地记录的 OpenCode session 已不存在，命令会自动清除失效映射、创建新会话并重试一次；
- 删除本地映射与删除 OpenCode session 是两个动作：正常清除会先删除 OpenCode session，再把本地记录移除。

## 权限与安全

`ocsh` 和 `occmt` 不自己实现 Shell 权限系统，完全使用 OpenCode 的 permission 配置。需要注意：

- `opencode run` 是非交互模式，遇到需要确认（`ask`）的权限会**自动拒绝**（auto-reject）；
- 因此需要放行的操作请在 OpenCode 配置中设为 `allow`，需要禁止的设为 `deny`；
- 只读调查（`ls`、`grep`、`git status` 等）是否能自动执行，也由这份配置决定。

## 调试

所有命令支持 `--debug`，会输出实际执行的 `opencode run` 参数与原始 JSON 事件（输出到 stderr）：

```bash
ocsh --debug "查看磁盘占用"
```

## 旧命令兼容

| 旧命令 | 新命令 |
| :--- | :--- |
| `aiq` / `aiask` | `ocq` |
| `aish` | `ocsh` |
| `ait` / `aitrans` | `oct` |

旧命令名仍可运行，但会提示一次更名信息。旧的 `~/.config/aiclikits/config.json`（`openai`、`models`、`defaults` 等）不再使用，provider / permission 由 OpenCode 管理；oclikits 自己只在 `~/.config/oclikits/config.jsonc` 保存模型配置。

## 开发

```bash
cargo build           # 构建
cargo test            # 单元测试
cargo build --release # 发布构建
```

代码结构：

```text
src/
├── main.rs             # 入口：argv[0] 别名分派与子命令分发
├── cli.rs              # clap 命令与参数定义
├── config.rs           # config.jsonc 读取与模型解析
├── error.rs            # 统一错误类型
├── i18n.rs             # 中英文提示
├── opencode/
│   ├── runner.rs       # 统一 opencode run 调用与事件输出
│   ├── event.rs        # NDJSON 事件解析
│   └── session.rs      # sessions.json 读写与 OpenCode session 删除
├── commands/
│   ├── q.rs            # ocq
│   ├── sq.rs           # ocsq
│   ├── trans.rs        # oct
│   ├── shell.rs        # ocsh
│   └── commit.rs       # occmt
└── ui/
    └── mod.rs          # 耗时、确认等终端输出
```

`docs/reference/occmt.sh` 是 `occmt` 重写前的 Shell 参考实现。
