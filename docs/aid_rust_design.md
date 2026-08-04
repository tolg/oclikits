# aid CLI（Rust 实现）设计与架构方案

> 版本：v1.0  
> 日期：2026-08-04  
> 状态：草案 / 评估通过  

---

## 1. 目标与定位

`aid`（及兼容别名 `aiq`, `aish`, `aitrans`）是用 **Rust** 构建的高性能、无外部依赖的 AI 命令行工具集。

### 核心设计原则
1. **零外部运行时依赖（Single Executable）**：彻底解耦 `zsh`、`curl`、`jq`，无需在目标机器配置运行环境。
2. **极致性能与毫秒级启动**：启动延迟低于 10ms，内存占用极小。
3. **原生支持 SSE 流式打字机输出**：边生成边打字，显著降低用户感知延迟。
4. **统一配置与全命令耗时显示**：保持兼容 `~/.config/aiclikits/config.json`，所有回复末尾自动附带 `(用时 X.XXs)`。
5. **软链接/多入口兼容**：单个二进制文件通过检测 `argv[0]` 或子命令，同时兼容 `aid q`、`aiq`、`aish`、`aitrans` 等多种调用形式。

---

## 2. 技术选型 (Crate Ecosystem)

| 维度 | 选用 Crate | 选型说明 |
| :--- | :--- | :--- |
| **语言** | Rust (2021 Edition) | 内存安全、零成本抽象、编译为静态二进制 |
| **CLI 解析** | `clap` (v4, derive) | 支持子命令、别名处理、自动生成补全与 `--help` |
| **异步运行时** | `tokio` | 高性能异步 IO，处理 HTTP 流式响应 |
| **HTTP 客户端** | `reqwest` (stream, json) | 标准 OpenSSL/native-tls 支持，完美支持 SSE 流 |
| **JSON 处理** | `serde` & `serde_json` | 强类型配置解析与 API Payload 构建 |
| **终端渲染/交互** | `indicatif` + `colored` | Spinner 思考动画、命令行色彩高亮 |
| **键盘输入** | `crossterm` / `dialoguer` | `aish` 命令执行前单键确认（`Y/n`） |
| **配置路径** | `dirs` | 跨平台自动识别用户配置目录 (`~/.config`) |

---

## 3. 项目代码目录架构

```text
aiclikits/ (Rust Workspace)
├── Cargo.toml
├── src/
│   ├── main.rs              # 程序入口：argv[0] 别名判定与 Clap 分发
│   ├── cli.rs               # Clap 命令与参数定义
│   ├── config.rs            # config.json 读写与兼容逻辑
│   ├── error.rs             # 统一 Error 枚举与友好提示格式化
│   ├── timer.rs             # 高精度耗时统计模块 (Execution Timer)
│   ├── client/              # OpenAI API 客户端
│   │   ├── mod.rs
│   │   ├── types.rs         # Request / Response 数据结构
│   │   └── sse.rs           # SSE 流式解析器 (Eventsource)
│   ├── commands/            # 命令业务逻辑
│   │   ├── mod.rs
│   │   ├── q.rs             # aiq / aiask (简洁问答 + 历史上下文)
│   │   ├── trans.rs         # aitrans / ait (中英翻译与错别字纠正)
│   │   └── shell.rs         # aish (自然语言转 Shell & 交互确认)
│   ├── history.rs           # 上下文历史文件读写与过期清理
│   └── ui.rs                # 终端 Spinner、颜色与分隔线渲染
└── scripts/
    └── install.sh           # 构建与软链接安装脚本
```

---

## 4. 核心功能与模块设计

### 4.1 二进制别名与子命令判定 (`main.rs`)
通过检查程序启动时的第一个参数（即 `argv[0]` 的文件名），支持两种使用习惯：
1. **多二进制别名模式**（通过 `ln -s aid aiq`）：
   - 当以 `aiq` 运行，等价于执行 `aid q`；
   - 当以 `aish` 运行，等价于执行 `aid shell`；
   - 当以 `ait` / `aitrans` 运行，等价于执行 `aid trans`。
2. **标准子命令模式**：
   - `aid q "swift 的 enum 特点"`
   - `aid shell "查找大文件"`

### 4.2 高精度耗时模块 (`timer.rs`)
基于 `std::time::Instant` 实现生命周期耗时追踪：
```rust
pub struct CommandTimer {
    start: std::time::Instant,
}

impl CommandTimer {
    pub fn start() -> Self {
        Self { start: std::time::Instant::now() }
    }

    pub fn print_elapsed(&self) {
        let elapsed = self.start.elapsed().as_secs_f64();
        println!("(用时 {:.2}s)", elapsed);
    }
}
```

### 4.3 SSE 流式打字机响应 (`client/sse.rs`)
在 `aiq` 和 `aitrans` 中，通过 `reqwest::bytes_stream()` 逐帧读取 OpenAI API 的 `data: {...}` 事件：
- 当开启流式模式时，AI 产生的每一个 token 立即打印到 `stdout` (`print!` + `io::stdout().flush()`)。
- 当 API 请求失败时，提取标准 `error.message` 打印友好提示。

### 4.4 问答历史管理 (`history.rs`)
- 兼容保存于 `~/.config/aiclikits/aiq_context.json`。
- 读取时自动判断文件修改时间（`mtime`），超过 28 小时自动作废旧历史。
- 提供 `-n / --new` 标记清空历史。

### 4.5 命令行交互确认 (`commands/shell.rs`)
- 系统提示词自动注入操作系统（`uname -s`）与当前 Shell 名称。
- 解析返回的 Command 并在终端加框高亮显示。
- 输出耗时 `(用时 X.XXs)` 后，等待用户输入 `Y/n`：
  - 按 `Enter` 或 `y` 自动将命令写入 Shell 历史并使用系统 Shell 执行；
  - 按 `n` 放弃执行。

---

## 5. 配置文件兼容性 (`config.rs`)

完全兼容现有 `~/.config/aiclikits/config.json` 结构：

```json
{
  "openai": {
    "apiKey": "sk-xxx",
    "baseUrl": "https://api.openai.com/v1"
  },
  "models": {
    "default": "qwen3-flash",
    "shell": "qwen3-coder-flash-2025-07-28",
    "ask": "qwen3-flash",
    "trans": "qwen3-flash"
  },
  "defaults": {
    "temperature": 0.1
  }
}
```
环境变量优先级：`OPENAI_API_KEY` 与 `OPENAI_BASE_URL` 可覆盖配置文件。

---

## 6. 构建与体积优化策略 (`Cargo.toml`)

为了保证 Rust 编译产物足够轻量（小于 5MB），配置 Release 编译选项：

```toml
[profile.release]
opt-level = "z"     # 优化体积
lto = true          # 链接时优化
codegen-units = 1   # 降低并行度提升体积压缩
strip = true        # 自动剥离符号表
panic = "abort"     # 移除 unwinding 逻辑
```

---

## 7. 迁移与实施步骤

1. **第一阶段：工程初始化与配置解析**
   - 创建 Rust 项目骨架，实现 `config.rs` 与 `clap` 路由。
2. **第二阶段：网络客户端与 SSE 流式输出**
   - 实现 Open API 调用的异步封装，支持普通及流式打字机打字。
3. **第三阶段：迁移 3 大核心命令**
   - 迁移 `aiq`（包含上下文历史管理与耗时）。
   - 迁移 `aitrans`（包含翻译规则与错别字纠正）。
   - 迁移 `aish`（包含单键交互确认与命令执行）。
4. **第四阶段：软链接与安装脚本更新**
   - 更新 `install.sh`，构建 `aid` 并创建 `aiq`, `aish`, `ait` 符号链接到 `~/.local/bin`。

---

## 8. 总结

采用 Rust 重构后，`aiclikits` 将获得：
- 真正的**单文件分发能力**（不需要预装 `jq` / `curl`）；
- **响应速度与打字机交互提升**；
- 保持现有的功能逻辑、配置格式及 `(用时 X.XXs)` 体验不变。
