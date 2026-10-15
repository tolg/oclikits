oclikits 实施设计

1. 项目定位

项目由 aiclikits 更名为：

oclikits

定位：

基于 OpenCode 的轻量命令行工具集，将 OpenCode 的 Agent、模型、工具和 Session 能力封装成适合日常终端工作流的短命令。

oclikits 不再实现自己的 LLM Client，不直接管理模型 API，也不自己保存完整对话历史。

职责缩减为：

1. 提供针对具体场景优化的 CLI。
2. 为不同命令构造合适的 OpenCode 任务。
3. 调用 opencode run。
4. 管理 OpenCode session 生命周期。
5. 管理命令权限和安全边界。
6. 对 OpenCode JSON 事件进行适合 CLI 的输出处理。

总体架构：

                         OpenCode
                            │
                    Agent / Tools / Go
                            │
                        oclikits
                            │
          ┌─────────┬───────┼───────┬─────────┐
          │         │       │       │         │
         ocq       ocsq    oct     ocsh      occmt
          │         │       │       │         │
       连续问答    单次问答  翻译   Shell Agent  Git提交

第一阶段暂不实现：

ocreview
ocfix
octest
...

以后根据实际重复使用场景再增加，不预先堆积命令。

⸻

2. 核心设计原则

2.1 OpenCode 是唯一 AI Runtime

旧架构：

aiclikits
   │
   ├── HTTP Client
   ├── API Key
   ├── Base URL
   ├── Model
   ├── Messages
   ├── History
   └── Shell execution

新架构：

oclikits
   │
   └── opencode
          │
          ├── Provider
          ├── OpenCode Go
          ├── Model
          ├── Agent
          ├── Tools
          ├── Permissions
          ├── Session
          ├── Context
          └── Compaction

因此逐步删除项目中不再需要的：

client/
history.rs
OpenAI API 配置
apiKey
baseUrl
temperature
ChatMessage

oclikits 不再重复实现 OpenCode 已经提供的能力。

⸻

3. Session 是核心抽象

oclikits 不保存：

user message
assistant message
tool call
tool result
...

这些全部属于 OpenCode。

oclikits 只保存：

某个逻辑上下文 → OpenCode sessionID

例如：

{
  "ocq": "ses_xxx",
  "ocsh": {
    "/Users/me": "ses_aaa",
    "/Users/me/project": "ses_bbb",
    "/tmp": "ses_ccc"
  }
}

建议统一保存：

~/.config/oclikits/sessions.json

示意：

{
  "version": 1,
  "ocq": {
    "session_id": "ses_xxx"
  },
  "ocsh": {
    "/Users/me": {
      "session_id": "ses_aaa"
    },
    "/Users/me/foofoil": {
      "session_id": "ses_bbb"
    }
  }
}

不要自己保存消息历史。

⸻

4. Session 生命周期分类

所有命令分成两类。

4.1 Persistent Session

需要连续上下文：

ocq
ocsh

生命周期：

创建
 ↓
反复复用
 ↓
OpenCode compact
 ↓
继续复用
 ↓
用户明确 -n / --new / --clear
 ↓
删除

⸻

4.2 Ephemeral Session

任务天然独立：

ocsq
oct
occmt

生命周期：

创建
 ↓
执行任务
 ↓
取得结果
 ↓
删除 session

即使任务失败，也应尽最大可能清理 session。

⸻

5. ocq —— 连续快速问答

5.1 定位

替代原：

aiq

用途：

ocq "Swift actor 和 class 有什么区别？"
ocq "那 actor 的性能开销呢？"
ocq "和刚才的 class 对比一下"

这三次调用属于同一个 OpenCode session。

⸻

5.2 Session

ocq 全局只有一个 active session。

不按目录区分。

原因：

ocq 是通用知识问答，而不是当前目录 Agent。

~/foo $ ocq "什么是 DoP？"
~/bar $ ocq "刚才说的封装发生在哪里？"

应该能够连续。

⸻

5.3 不再限制 10 轮

旧 aiq：

最多10轮
↓
删除最旧消息

新 ocq 不这么做。

改为：

OpenCode session
      ↓
context 增长
      ↓
OpenCode compaction
      ↓
继续 session

原则：

优先使用 OpenCode 自己的自动 compaction。

不要自己按照：

10轮
20轮
30轮

判断是否 compact。

原因是轮数和 token 数量没有稳定关系。

如果后续验证 OpenCode 的 run --session 场景不会自动 compact，再实现显式 compaction；在验证之前不要自行增加一套摘要逻辑。

⸻

5.4 清空机制

支持：

ocq -n "换个问题"
ocq --new "换个问题"

行为：

存在旧 session
 ↓
删除旧 session
 ↓
创建新 session
 ↓
执行问题
 ↓
保存新 sessionID

另外支持：

ocq --clear

行为：

删除当前 ocq session
 ↓
删除本地 sessionID
 ↓
退出

输出：

问答历史已清除。

⸻

6. ocsq —— Single Question

新增：

ocsq

全称：

OpenCode Single Question

用途：

ocsq "HTTP 409 和 422 有什么区别？"

特点：

无历史
无连续上下文
一次调用
一次回答

生命周期：

opencode run
 ↓
得到 sessionID
 ↓
输出回答
 ↓
opencode session delete

即：

Q → A → delete

适用于绝大多数不值得污染 ocq 历史的小问题。

⸻

6.1 与 ocq 的边界

ocsq
“这个问题以后不会继续讨论”

例如：

ocsq "tar -C 是什么意思？"

而：

ocq
“这个主题我可能继续问”

例如：

ocq "解释一下 Swift Concurrency 的 actor"
ocq "那 MainActor 呢？"
ocq "和 GCD 对比呢？"

⸻

7. oct —— 翻译

替代：

ait
aitrans

主命令：

oct

可考虑保留：

octrans

作为兼容 alias，但不是必要入口。

⸻

7.1 行为

继续保持现有翻译规则：

* 自动判断中英文。
* 单词返回多个高质量常用译法。
* 句子返回最佳译法。
* 英文拼写错误可纠正。
* 不输出不必要解释。

这些 prompt 规则可以直接从现有 trans.rs 迁移。

区别只是：

旧：

Rust
 ↓
ApiClient
 ↓
LLM API

新：

Rust
 ↓
opencode run
 ↓
OpenCode

⸻

7.2 Session

翻译必须是 ephemeral：

创建
 ↓
翻译
 ↓
输出
 ↓
删除

不同翻译之间绝不共享历史。

否则历史上下文可能影响译词选择。

⸻

8. ocsh —— Shell Agent

这是迁移中变化最大的工具。

旧：

aish

本质：

自然语言
 ↓
LLM生成一条shell
 ↓
用户确认
 ↓
Rust执行

新：

ocsh

本质：

当前目录下长期存在的轻量 OpenCode Shell Agent。

⸻

9. ocsh Session 模型

9.1 每个目录只能有一个 session

这是明确约束。

不提供：

ocsh --session xxx
ocsh --new-session-with-name
ocsh session list

也不做：

同目录多个 session

否则 oclikits 就开始重新实现 OpenCode TUI。

规则非常简单：

canonical working directory
        ↓
唯一 sessionID

例如：

/Users/me/foo
    → ses_A
/Users/me/bar
    → ses_B
/tmp
    → ses_C

⸻

9.2 为什么按目录

Shell 操作天然和当前工作目录绑定。

例如：

cd ~/project
ocsh "看看为什么启动失败"

之后：

ocsh "刚才那个错误是不是配置导致的？"

应该继续原 session。

但是：

cd /tmp
ocsh "看看哪些东西占空间"

不应该继承项目 session。

⸻

9.3 路径必须 canonicalize

不要直接用：

pwd

作为 key。

需要：

std::fs::canonicalize(...)

避免：

~/foo
/Users/me/foo
/Users/me/foo/.

产生多个 session。

⸻

10. ocsh 历史与 Compaction

和 ocq 一样：

不自己保存历史消息。

OpenCode session 是唯一历史来源。

也不设置：

10轮自动删除

而是：

session
 ↓
不断使用
 ↓
OpenCode自动compact
 ↓
继续使用

如果经过实际验证发现 opencode run --session 不会触发自动 compaction，再增加显式 compact 支持。

不要提前自己实现 summary。

⸻

11. ocsh 手动清除

支持：

ocsh -n "重新调查这个目录"
ocsh --new "重新调查这个目录"

行为：

当前目录存在 session
 ↓
delete
 ↓
新建
 ↓
执行任务

以及：

ocsh --clear

只删除：

当前目录对应 session

绝不影响其他目录。

例如：

~/foo → ses_A
~/bar → ses_B

在 foo：

ocsh --clear

只删除：

ses_A

ses_B 保留。

⸻

12. ocsh 安全模型

旧 aish 的优点是：

模型不能直接执行
↓
用户确认命令
↓
Rust执行

迁移 OpenCode 后不能简单丢掉这个安全边界。

但是也不应该继续要求：

每条 shell 都确认

否则 Agent 多步骤执行能力几乎失去意义。

推荐策略：

只读操作

默认允许：

pwd
ls
find
grep
rg
cat
head
tail
du
df
ps
lsof
git status
git diff
docker ps
...

Agent 可以自动调查。

有副作用操作

由 OpenCode permission 进行确认。

例如：

rm
mv
kill
git commit
git reset
docker rm
docker prune
brew install
...

形成：

用户：
ocsh "看看8080是谁占的，如果没用就停掉"
Agent：
lsof ...
ps ...
        ↓
需要 kill
        ↓
permission ask
        ↓
用户确认
        ↓
kill
        ↓
再次验证

具体 permission 配置应作为实现阶段单独验证项。

原则：

不自己重新实现一套 Shell 权限系统，优先使用 OpenCode permission。

⸻

13. occmt —— Git Commit

occmt 纳入 oclikits，但第一阶段不重新设计功能。

现有 Shell 版本作为：

reference/
occmt.sh

或：

docs/reference/occmt.sh

用于 Rust 实现参考。

最终必须重写成 Rust，与其他命令共享：

OpenCode runner
Session manager
JSON event parser
CLI
错误处理
模型配置

不长期保留 Shell 实现作为正式命令。

⸻

14. occmt 功能要求

保持已经确定的行为：

occmt

作用于当前目录。

如果当前目录是 Git repo：

只处理当前 repo

否则搜索：

一级子目录
二级子目录

最多两级。

多个存在修改的 repo：

列出
 ↓
用户确认
 ↓
逐个处理

提交：

Conventional Commits

语言：

默认：
遵循 AGENTS.md / 项目上下文
--en：
英文
--zh：
简体中文

模型默认：

opencode-go/mimo-v2.6-flash

支持：

occmt -m provider/model

不：

push
amend

每个 repo：

独立 ephemeral session
 ↓
commit
 ↓
删除 session

⸻

15. 模型配置

模型不写死在代码里。由 `~/.config/oclikits/config.jsonc` 提供全局默认值，
并可为各命令配置可选的专用模型：

{
  // 支持 // 与 /* */ 注释（JSONC）
  "model": "opencode-go/mimo-v2.6-flash",
  "commands": {
    "q": "",
    "sq": "",
    "trans": "",
    "shell": "",
    "commit": ""
  }
}

commands 的键为命令规范名，留空则回退到全局 model。
旧 aiclikits 的 models 结构（default / shell / ask / trans）不再使用：

{
  "models": {
    "shell": "...",
    "ask": "...",
    "trans": "..."
  }
}

OpenCode 已经负责 provider。

所有命令统一支持临时覆盖：

-m <model>
--model <model>

例如：

ocq -m opencode-go/deepseek-v4.1-flash "..."

解析优先级：-m/--model > commands.<命令> > 全局 model > 报错并提示配置。
未配置模型时，命令报错退出；若配置文件不存在，会先写入带注释的模板。

⸻

16. 配置文件

旧：

~/.config/aiclikits/config.json

迁移为：

~/.config/oclikits/

包含：

config.jsonc     # 用户维护：全局 model + 各命令专用 model（安装时生成带注释模板，运行时兜底）
sessions.json    # 自动维护：逻辑上下文 → OpenCode sessionID

config.jsonc 必须至少配置全局 model，否则命令会报错并提示配置方法。
配置文件为 JSONC（JSON + 注释），解析前会剥离注释。

OpenCode 自己已经有：

provider
permission
agent
MCP
...

配置。

oclikits 只保存自己的模型配置，不复制这些配置。

⸻

17. Rust 模块设计

建议重构为：

src/
├── main.rs
├── cli.rs
├── error.rs
├── i18n.rs
│
├── opencode/
│   ├── mod.rs
│   ├── runner.rs
│   ├── event.rs
│   └── session.rs
│
├── commands/
│   ├── mod.rs
│   ├── q.rs
│   ├── sq.rs
│   ├── trans.rs
│   ├── shell.rs
│   └── commit.rs
│
└── ui/
    └── ...

删除或逐步淘汰：

client/
history.rs

⸻

18. OpenCode Runner

所有命令不得自己：

Command::new("opencode")

各写一套。

统一实现：

OpenCodeRunner

概念 API：

struct RunOptions {
    model: Option<String>,
    session_id: Option<String>,
    directory: Option<PathBuf>,
    prompt: String,
}
struct RunResult {
    session_id: String,
    text: String,
    exit_code: i32,
}

调用：

runner.run(options).await

内部统一：

opencode run
--format json
--model ...
--session ...
--dir ...
prompt

⸻

19. JSON Event Parser

根据当前已经实测的 OpenCode 输出：

{
  "type": "step_start",
  "sessionID": "ses_xxx"
}

以及：

{
  "type": "text",
  "sessionID": "ses_xxx",
  "part": {
    "type": "text",
    "text": "hello"
  }
}

统一解析：

#[derive(Deserialize)]
struct OpenCodeEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(rename = "sessionID")]
    session_id: Option<String>,
    part: Option<Part>,
}

Runner 负责：

读取 stdout NDJSON
 ↓
第一条 sessionID → 保存
 ↓
text event → 流式输出/收集
 ↓
其他事件按需要处理

命令模块不应该知道 JSON schema。

⸻

20. SessionManager

统一：

SessionManager

职责：

load
save
get_ocq
set_ocq
clear_ocq
get_ocsh(path)
set_ocsh(path, session)
clear_ocsh(path)
delete_opencode_session(sessionID)

注意两个动作不同：

删除本地映射

和：

删除 OpenCode session

正常清除必须：

OpenCode delete 成功
 ↓
删除本地记录

如果 OpenCode session 已不存在：

视为已经清除
 ↓
删除本地 stale mapping

⸻

21. 并发和原子写入

sessions.json 不要直接覆盖写。

至少：

写 sessions.json.tmp
 ↓
fsync/flush
 ↓
rename

避免进程中断导致 session 文件损坏。

如果实现成本不高，增加文件锁。

典型风险：

terminal A:
ocq ...
terminal B:
ocsh ...
同时修改 sessions.json

不能互相覆盖。

⸻

22. Session 异常恢复

例如：

sessions.json
↓
ses_123

但 OpenCode 中已经删除。

下一次：

ocq "..."

OpenCode 返回 session 不存在。

处理：

检测 session invalid
 ↓
删除 stale mapping
 ↓
创建新 session
 ↓
重新执行当前请求一次

只自动 retry 一次。

避免无限重试。

⸻

23. CLI 设计

统一风格。

ocq

ocq [OPTIONS] <QUESTION>
-n, --new
    删除旧问答 session，开始新会话
--clear
    清除问答 session 后退出
-m, --model <MODEL>
    指定模型
-h, --help

⸻

ocsq

ocsq [OPTIONS] <QUESTION>
-m, --model <MODEL>
-h, --help

不需要：

-n
--clear

因为永远无状态。

⸻

oct

oct [OPTIONS] <TEXT>
-m, --model <MODEL>
-h, --help

无 session 参数。

⸻

ocsh

ocsh [OPTIONS] <TASK>
-n, --new
    清除当前目录 session 后执行任务
--clear
    清除当前目录 session 后退出
-m, --model <MODEL>
-h, --help

明确不提供：

--session
--list
--switch

因为：

一个目录只能有一个 ocsh session。

需要复杂 session 管理时直接使用 OpenCode TUI/Web。

⸻

occmt

occmt [OPTIONS]
--en
--zh
-m, --model <MODEL>
-h, --help

⸻

24. Binary / Alias 方案

继续沿用现项目通过 executable name 分派的方式是合理的。

安装一个主 binary：

oc

然后 symlink：

ocq    → oc
ocsq   → oc
oct    → oc
ocsh   → oc
occmt  → oc

也允许：

oc q
oc sq
oc trans
oc shell
oc commit

但主要用户入口仍然是短命令：

ocq
ocsq
oct
ocsh
occmt

⸻

25. 旧命令兼容

迁移初期可以保留：

aiq
aish
ait
aitrans

symlink 到新 binary，并输出一次迁移提示：

提示：aiq 已更名为 ocq。

但不建议永久维护。

可以在一个明确版本后删除。

⸻

26. Prompt 组织

不要把所有 prompt 硬编码进 main.rs。

每个 command 自己维护 system/task prompt：

commands/q.rs
commands/sq.rs
commands/trans.rs
commands/shell.rs
commands/commit.rs

公共约束尽量少。

原因：

ocq

需要简洁问答；

oct

需要严格输出；

ocsh

需要 Agent 行为；

occmt

需要 Git 操作。

强行抽象成一个“大 system prompt”反而降低可维护性。

⸻

27. 输出原则

oclikits 是 CLI，不应该把 OpenCode 内部事件全部泄露给用户。

默认只显示：

Agent text
permission request
必要错误
最终结果

隐藏：

step_start
messageID
partID
sessionID
token metadata

提供调试模式：

--debug

才输出原始信息。

⸻

28. 效率原则

这是整个项目的重要目标。

不启动 TUI

全部使用：

opencode run

不使用 standalone

默认复用 OpenCode V2 service。

不重复分析

例如 occmt：

Shell/Rust负责：
repo discovery
git dirty detection
multi-repo confirmation

不要让模型做这些确定性工作。

模型只负责：

理解diff
判断提交内容
生成commit message
执行commit

简单任务使用快速模型

默认：

opencode-go/mimo-v2.6-flash

不是为了追求最高模型能力，而是：

低延迟
+
足够能力
+
OpenCode Agent Harness

⸻

29. 第一阶段实施顺序

建议严格按下面顺序。

Phase 1：OpenCode 基础层

实现：

opencode/event.rs
opencode/runner.rs
opencode/session.rs

先写一个内部测试：

run("只回答 hello")

确保能够：

启动
解析 text
获得 sessionID
继续 session
删除 session

这是整个项目基础。

⸻

Phase 2：ocsq

第一个迁移：

ocsq

因为最简单：

run
 ↓
output
 ↓
delete

用它验证 ephemeral session。

⸻

Phase 3：oct

迁移现有 trans.rs prompt。

验证：

严格输出
流式输出
ephemeral cleanup

完成后可以删除旧 API 翻译实现。

⸻

Phase 4：ocq

实现：

persistent session
-n
--clear
stale session recovery

验证 OpenCode automatic compaction。

确认之后删除旧：

history.rs

⸻

Phase 5：ocsh

这是最需要测试的阶段。

实现：

directory → session
-n
--clear
permission
multi-step Agent

重点验证：

只读调查
危险操作确认
跨调用连续上下文
不同目录隔离
compaction

⸻

Phase 6：occmt

把现有：

occmt.sh

作为行为规格参考。

Rust 重写：

repo discovery
confirmation
language
model
OpenCode run
session cleanup

完成后删除正式 Shell 入口。

⸻

Phase 7：清理旧架构

确认所有命令稳定后删除：

ApiClient
OpenAI-compatible client
history.rs
apiKey
baseUrl
temperature
旧 models 配置

项目完成从：

AI CLI Kits

到：

OpenCode CLI Kits

的迁移。

⸻

30. 第一版最终形态

oclikits
│
├── ocq
│    └── 全局长期 session
│         └── OpenCode compaction
│
├── ocsq
│    └── 临时 session → delete
│
├── oct
│    └── 临时 session → delete
│
├── ocsh
│    └── 每目录唯一长期 session
│         └── OpenCode compaction
│
└── occmt
     └── 每 repo 临时 session → delete

核心区别可以概括成：

            是否需要上下文？
                 │
          ┌──────┴──────┐
          是             否
          │              │
       persistent     ephemeral
          │              │
     ┌────┴────┐    ┌────┼────┐
    ocq       ocsh  ocsq oct  occmt

⸻

31. 明确不做的事情

第一版尤其要控制范围。

不做：

自己的 LLM API Client
自己的聊天历史
自己的 context summary
自己的 Agent framework
自己的 tool calling
自己的 shell execution agent
复杂 session UI
session list/switch
多个 ocsh session/目录
ocreview
ocfix
octest
Web UI
TUI

这些要么 OpenCode 已经提供，要么目前没有足够实际需求。

oclikits 的价值不在于成为“小 OpenCode”，而在于：

把 OpenCode 已有能力压缩成几个低摩擦、符合 Unix 使用习惯、可以随手调用的命令。

这是第一版最重要的产品边界。
