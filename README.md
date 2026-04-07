# aiclikits

一组可扩展的 AI 命令行工具（OpenAI API 兼容）。

## 安装

```bash
./install.sh
```

## 配置文件

位置：`~/.config/aiclikits/config.json`

示例见仓库根目录 `config.example.json`。

示例：

```json
{
  "openai": {
    "apiKey": "sk-your-key",
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

说明：

- `openai.apiKey`：必填
- `openai.baseUrl`：可选，不填时默认 `https://api.openai.com/v1`

## 命令

### aish

自然语言转 shell 命令，并在执行前确认。

```bash
aish "列出当前目录下最大的 10 个文件"
```

### aiq

简洁回答常识问题（1-3 句话）。

```bash
aiq "swift 语言的枚举什么样"
```

会话历史：

- `aiq` 会将最近的对话历史保存在 `~/.config/aiclikits/aiq_context.json`
- 默认会带上历史上下文连续对话
- 历史最多保留 20 条消息，即最多 10 轮用户/助手对话
- 历史中只保存用户消息和助手回复，系统提示词不会写入历史文件
- 如果历史文件损坏或不是合法 JSON，会自动回退为空历史
- 使用 `-n` 或 `--new` 可清空历史并开始新对话

示例：

```bash
aiq -n "重新开始，解释一下 Swift 的 enum"
```

兼容说明：`aiask` 仍可用，但已转发到 `aiq`。

### aitrans / ait

中英互译。自动判断输入语言，并翻译成另一种语言。

```bash
aitrans "apple"
ait "今天天气不错"
```

规则：

- 如果输入是单词，返回常用、自然、现代语境里的译词，每行一个
- 单词翻译优先质量，不强求凑满数量；如果常用译词本来就少，就少返回几个
- 不返回明显生僻、过时、书面化过强、领域化、音译硬凑或不自然的候选
- 如果输入是句子、短语或多词表达，返回唯一的最佳翻译结果
- 不保留会话历史，每次都是独立翻译
- 使用 `models.trans` 作为模型配置；未设置时回退到 `models.default`
