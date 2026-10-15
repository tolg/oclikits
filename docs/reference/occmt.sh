#!/usr/bin/env bash

# occmt - 使用 OpenCode 自动提交 Git 修改
#
# 依赖：
#   - git
#   - opencode v2
#   - jq
#
# 每个仓库执行完成后会自动删除本次创建的 OpenCode session。

set -uo pipefail

readonly PROGRAM="occmt"
readonly DEFAULT_MODEL="opencode-go/mimo-v2.6-flash"

MODEL="$DEFAULT_MODEL"
LANGUAGE=""
ROOT="$(pwd)"

usage() {
  cat <<'EOF'
用法：
  occmt [选项]

使用 OpenCode 分析当前 Git 仓库中的未提交修改，并按照
Conventional Commits 规范自动创建提交。

如果当前目录本身不是 Git 仓库，则向下搜索最多两级子目录中的
Git 仓库。发现多个存在未提交修改的仓库时，会列出这些仓库并在
执行前要求确认。

选项：
  --en
      强制使用英文编写提交信息。

  --zh
      强制使用简体中文编写提交信息。
      Conventional Commits 的类型仍使用标准英文形式，
      例如 feat、fix、refactor、docs、test、chore。

  -m, --model <模型>
      指定 OpenCode 使用的模型。

      默认：
        opencode-go/mimo-v2.6-flash

  -h, --help
      显示此帮助信息并退出。

行为：
  - 始终作用于执行命令时所在的当前目录。
  - 如果当前目录位于 Git 仓库内，只处理当前 Git 仓库。
  - 如果当前目录不属于 Git 仓库，则搜索最多两级子目录。
  - 只处理存在未提交修改的 Git 仓库。
  - 发现多个需要提交的仓库时，执行前要求用户确认。
  - OpenCode 会读取并遵循项目中的 AGENTS.md 等项目指令。
  - 未指定 --en 或 --zh 时，不强制提交信息语言，由项目指令
    （例如 AGENTS.md）及项目上下文决定。
  - 提交信息遵循 Conventional Commits 规范。
  - 不执行 git push。
  - 不修改、覆盖或 amend 已有提交。
  - 不创建空提交。
  - 不要求 Agent 再次确认提交。
  - 每个仓库使用独立的 OpenCode session。
  - 任务结束后自动删除本次创建的 OpenCode session。
  - 一个仓库默认只创建一个提交。

示例：
  occmt

  occmt --zh

  occmt --en

  occmt -m opencode-go/mimo-v2.6-flash

  occmt --zh -m provider/model
EOF
}

die() {
  printf 'occmt：%s\n' "$*" >&2
  exit 1
}

# ----------------------------------------------------------------------
# 参数解析
# ----------------------------------------------------------------------

while (($#)); do
  case "$1" in
    --en)
      [[ "$LANGUAGE" != "zh" ]] ||
        die "--en 和 --zh 不能同时使用"
      LANGUAGE="en"
      shift
      ;;

    --zh)
      [[ "$LANGUAGE" != "en" ]] ||
        die "--en 和 --zh 不能同时使用"
      LANGUAGE="zh"
      shift
      ;;

    -m|--model)
      (($# >= 2)) ||
        die "$1 后必须指定模型名称"
      MODEL="$2"
      shift 2
      ;;

    -h|--help)
      usage
      exit 0
      ;;

    --)
      shift
      (($# == 0)) ||
        die "无法识别的参数：$1"
      ;;

    -*)
      die "未知选项：$1（使用 -h 查看帮助）"
      ;;

    *)
      die "无法识别的参数：$1（使用 -h 查看帮助）"
      ;;
  esac
done

# ----------------------------------------------------------------------
# 检查依赖
# ----------------------------------------------------------------------

command -v git >/dev/null 2>&1 ||
  die "未找到 git，请先安装或检查 PATH"

command -v opencode >/dev/null 2>&1 ||
  die "未找到 opencode，请先安装或检查 PATH"

command -v jq >/dev/null 2>&1 ||
  die "未找到 jq，请先安装或检查 PATH"

# ----------------------------------------------------------------------
# 查找 Git 仓库
# ----------------------------------------------------------------------

repos=()

# 如果当前目录本身位于某个 Git worktree 内，只处理这个仓库。
# 这样可以避免在项目内部执行 occmt 时意外提交嵌套仓库。
if git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then

  repo_root="$(git -C "$ROOT" rev-parse --show-toplevel)"
  repos+=("$repo_root")

else

  # 当前目录不是 Git 仓库时，搜索：
  #
  #   ./project/.git
  #   ./group/project/.git
  #
  # 即项目根目录最多位于当前目录以下两级。
  #
  # Git worktree 的 .git 可能是文件而不是目录，因此只按名称查找。

  while IFS= read -r -d '' git_entry; do

    repo="${git_entry%/.git}"

    if git -C "$repo" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
      repos+=("$repo")
    fi

  done < <(
    find "$ROOT" \
      -mindepth 2 \
      -maxdepth 3 \
      -name .git \
      -print0 2>/dev/null
  )

fi

((${#repos[@]} > 0)) ||
  die "当前目录及其两级子目录内没有找到 Git 仓库"

# ----------------------------------------------------------------------
# 去重
# ----------------------------------------------------------------------

unique_repos=()
declare -A seen_repos

for repo in "${repos[@]}"; do

  canonical="$(cd "$repo" 2>/dev/null && pwd -P)" ||
    continue

  if [[ -z "${seen_repos[$canonical]+x}" ]]; then
    seen_repos["$canonical"]=1
    unique_repos+=("$canonical")
  fi

done

repos=("${unique_repos[@]}")

# ----------------------------------------------------------------------
# 只保留存在修改的仓库
# ----------------------------------------------------------------------

changed_repos=()

for repo in "${repos[@]}"; do

  if [[ -n "$(git -C "$repo" status --porcelain --untracked-files=normal)" ]]; then
    changed_repos+=("$repo")
  fi

done

if ((${#changed_repos[@]} == 0)); then
  printf '没有需要提交的修改。\n'
  exit 0
fi

# ----------------------------------------------------------------------
# 多仓库确认
# ----------------------------------------------------------------------

if ((${#changed_repos[@]} > 1)); then

  printf '发现 %d 个存在未提交修改的 Git 仓库：\n\n' \
    "${#changed_repos[@]}"

  for repo in "${changed_repos[@]}"; do

    if [[ "$repo" == "$ROOT/"* ]]; then
      display="./${repo#"$ROOT/"}"
    else
      display="$repo"
    fi

    printf '  - %s\n' "$display"

  done

  printf '\n是否提交以上所有仓库？[y/N] '
  read -r answer

  case "$answer" in
    y|Y|yes|YES|Yes)
      ;;
    *)
      printf '已取消。\n'
      exit 0
      ;;
  esac

fi

# ----------------------------------------------------------------------
# 构造 Agent Prompt
# ----------------------------------------------------------------------

build_prompt() {

  local language_instruction

  case "$LANGUAGE" in

    en)
      language_instruction='Write the commit message in English.'
      ;;

    zh)
      language_instruction='Write the commit message in Simplified Chinese. Keep the Conventional Commits type in its standard English form, such as feat, fix, refactor, docs, test, or chore.'
      ;;

    *)
      language_instruction='Do not impose a commit-message language yourself. Follow the repository instructions, especially AGENTS.md, if they specify one; otherwise infer the appropriate language from the repository context.'
      ;;

  esac

  cat <<EOF
Commit the current repository changes.

Requirements:

1. Inspect and obey repository instructions, including AGENTS.md when present.

2. Inspect Git status and the current diff efficiently. Read only additional
   files that are necessary to understand the changes.

3. Create exactly one appropriate commit for the current uncommitted changes.

4. The commit message MUST follow the Conventional Commits specification.

5. $language_instruction

6. Stage tracked and untracked files that clearly belong to the current change.
   Do NOT stage obviously unrelated files, generated files, temporary files,
   credentials, secrets, dependency directories, or build output.

7. Do not modify source files merely to improve the commit.

8. Do not push.

9. Do not amend, rewrite, squash, or otherwise modify existing commits.

10. Do not create an empty commit.

11. Do not ask for confirmation. The user has already authorized this command
    to create the commit.

12. After committing, briefly report the resulting commit hash and subject.

Be efficient. Inspect the existing changes, determine an appropriate
Conventional Commit message, stage the appropriate files, create one commit,
and stop.
EOF
}

PROMPT="$(build_prompt)"

# ----------------------------------------------------------------------
# Session 管理
# ----------------------------------------------------------------------

CURRENT_SESSION=""

cleanup_session() {

  if [[ -n "$CURRENT_SESSION" ]]; then

    opencode session delete "$CURRENT_SESSION" >/dev/null 2>&1 || {
      printf 'occmt：警告：无法删除 OpenCode session：%s\n' \
        "$CURRENT_SESSION" >&2
    }

    CURRENT_SESSION=""

  fi
}

# Ctrl+C / kill 时尽量清理当前 session。

trap 'cleanup_session; exit 130' INT
trap 'cleanup_session; exit 143' TERM
trap 'cleanup_session' EXIT

# ----------------------------------------------------------------------
# 提交单个仓库
# ----------------------------------------------------------------------

commit_repo() {

  local repo="$1"
  local tmp
  local status
  local session_id
  local display

  if [[ "$repo" == "$ROOT" ]]; then

    display="."

  elif [[ "$repo" == "$ROOT/"* ]]; then

    display="./${repo#"$ROOT/"}"

  else

    display="$repo"

  fi

  printf '\n正在处理：%s\n' "$display"

  tmp="$(mktemp -t occmt.XXXXXX)" ||
    die "无法创建临时文件"

  CURRENT_SESSION=""

  # 使用 JSON 输出的原因：
  #
  # 1. 可以精确获取本次 run 创建的 sessionID；
  # 2. 不需要通过 session list 猜测最新 session；
  # 3. 即使同时存在其他 OpenCode 任务，也不会误删其他 session。
  #
  # tee 保存完整事件流。
  # jq 只把 Agent 的文本回复显示给用户。

  opencode run \
    --dir "$repo" \
    --model "$MODEL" \
    --format json \
    "$PROMPT" \
    | tee "$tmp" \
    | jq -r '
        select(
          .type == "text"
          and .part.text != null
        )
        | .part.text
      '

  status=${PIPESTATUS[0]}

  # sessionID 位于 OpenCode JSON 事件顶层：
  #
  # {
  #   "type": "text",
  #   "sessionID": "ses_xxx",
  #   ...
  # }

  session_id="$(
    jq -r '
      select(.sessionID != null)
      | .sessionID
    ' "$tmp" 2>/dev/null | head -n 1
  )"

  rm -f "$tmp"

  # ------------------------------------------------------------------
  # 删除本次 OpenCode session
  # ------------------------------------------------------------------

  if [[ -n "$session_id" ]]; then

    CURRENT_SESSION="$session_id"
    cleanup_session

  else

    printf 'occmt：警告：未能获取本次 OpenCode session ID，无法自动清理 session。\n' \
      >&2

  fi

  # ------------------------------------------------------------------
  # 检查 OpenCode 执行结果
  # ------------------------------------------------------------------

  if ((status != 0)); then

    printf 'occmt：处理 %s 时 OpenCode 执行失败（退出码 %d）。\n' \
      "$display" "$status" >&2

    return "$status"

  fi

  # ------------------------------------------------------------------
  # 显示最终提交
  # ------------------------------------------------------------------

  printf '提交结果：'
  git -C "$repo" log -1 --format='%h %s'

  return 0
}

# ----------------------------------------------------------------------
# 主流程
# ----------------------------------------------------------------------

failures=0

for repo in "${changed_repos[@]}"; do

  if ! commit_repo "$repo"; then
    ((failures++))
  fi

done

printf '\n'

if ((failures > 0)); then

  printf '处理完成，其中 %d 个仓库提交失败。\n' \
    "$failures" >&2

  exit 1

fi

if ((${#changed_repos[@]} == 1)); then

  printf '提交完成。未执行 push。\n'

else

  printf '全部 %d 个仓库处理完成。未执行 push。\n' \
    "${#changed_repos[@]}"

fi
