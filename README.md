# aiclikits

一组可扩展的 AI 命令行工具（OpenAI API 兼容）。

## 安装

```bash
./install.sh
```

## 环境变量

- `OPENAI_API_KEY`（必填）
- `OPENAI_BASE_URL`（可选，用于中转，示例：`https://your-proxy.com/v1`）

示例：
```bash
export OPENAI_API_KEY="sk-your-key"
export OPENAI_BASE_URL="https://your-proxy.com/v1"
```

## 配置文件

位置：`~/.config/aiclikits/config.json`

示例见仓库根目录 `config.example.json`。

## 命令

### aish

自然语言转 shell 命令，并在执行前确认。

```bash
aish "列出当前目录下最大的 10 个文件"
```

### aiask

简洁回答常识问题（1-3 句话）。

```bash
aiask "swift 语言的枚举什么样"
```
