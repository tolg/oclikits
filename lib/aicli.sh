#!/bin/zsh

zmodload zsh/datetime 2>/dev/null || true

AICLI_CONFIG_DIR="${HOME}/.config/aiclikits"
AICLI_CONFIG_FILE="${AICLI_CONFIG_DIR}/config.json"

_AICLI_START_TIME=""

aicli_start_timer() {
  _AICLI_START_TIME="${EPOCHREALTIME:-}"
}

aicli_elapsed_sec() {
  if [[ -n "$_AICLI_START_TIME" && -n "$EPOCHREALTIME" ]]; then
    local elapsed=$(( EPOCHREALTIME - _AICLI_START_TIME ))
    printf "%.2fs" "$elapsed"
  else
    printf "0.00s"
  fi
}

aicli_print_elapsed() {
  local elapsed
  elapsed="$(aicli_elapsed_sec)"
  echo "(用时 ${elapsed})"
}

aicli_start_timer


_aicli_err() {
  echo "$*" 1>&2
}

_aicli_die() {
  _aicli_err "$*"
  exit 1
}

_aicli_check_deps() {
  command -v jq >/dev/null 2>&1 || _aicli_die "❌ 错误: 未安装 jq，请先安装: brew install jq"
  command -v curl >/dev/null 2>&1 || _aicli_die "❌ 错误: 未安装 curl"
}

_aicli_cfg_get() {
  local jq_expr="$1"
  [[ -f "$AICLI_CONFIG_FILE" ]] || return 0
  jq -r "${jq_expr} // empty" "$AICLI_CONFIG_FILE" 2>/dev/null || true
}

_aicli_trim() {
  local s="$1"
  s="${s#${s%%[![:space:]]*}}"
  s="${s%${s##*[![:space:]]}}"
  printf '%s' "$s"
}

_aicli_json_compact() {
  local raw="$1"
  if [[ -z "$raw" ]]; then
    return 1
  fi
  printf '%s' "$raw" | jq -c . 2>/dev/null
}

aicli_api_key() {
  local key
  key="$(_aicli_cfg_get ".openai.apiKey")"
  key="$(_aicli_trim "$key")"
  printf '%s' "$key"
}

aicli_base_url() {
  local base
  base="$(_aicli_cfg_get ".openai.baseUrl")"
  base="$(_aicli_trim "$base")"
  if [[ -z "$base" ]]; then
    base="https://api.openai.com/v1"
  fi
  base="${base%/}"
  printf '%s' "$base"
}

aicli_model_for() {
  local tool="$1"
  local fallback="$2"
  local model
  model="$(_aicli_cfg_get ".models.${tool}")"
  model="$(_aicli_trim "$model")"
  if [[ -z "$model" ]]; then
    model="$(_aicli_cfg_get ".models.default")"
    model="$(_aicli_trim "$model")"
  fi
  if [[ -z "$model" ]]; then
    model="$fallback"
  fi
  printf '%s' "$model"
}

aicli_temperature() {
  local t
  t="$(_aicli_cfg_get ".defaults.temperature")"
  t="$(_aicli_trim "$t")"
  if [[ -z "$t" ]]; then
    t="0.1"
  fi
  printf '%s' "$t"
}

aicli_chat_messages() {
  _aicli_check_deps

  local model="$1"
  local messages_json="$2"
  local temperature="${3:-}"

  if [[ -z "$temperature" ]]; then
    temperature="$(aicli_temperature)"
  fi

  local messages_compact
  messages_compact="$(_aicli_json_compact "$messages_json")" || _aicli_die "❌ 错误: 传入的 messages 不是合法 JSON"

  local temperature_json
  temperature_json="$(printf '%s' "$temperature" | jq -R 'fromjson?' 2>/dev/null)"
  if [[ -z "$temperature_json" || "$temperature_json" == "null" ]]; then
    temperature_json="0.1"
  fi

  local api_key
  api_key="$(aicli_api_key)"
  [[ -n "$api_key" ]] || _aicli_die "❌ 错误: 未在配置文件设置 openai.apiKey (${AICLI_CONFIG_FILE})"

  local base url
  base="$(aicli_base_url)"
  url="${base}/chat/completions"

  local payload
  payload=$(jq -n \
    --arg model "$model" \
    --argjson messages "$messages_compact" \
    --argjson temperature "$temperature_json" \
    '{
      model: $model,
      messages: $messages,
      temperature: $temperature
    }')

  local marker response http_code body
  marker="AICLI_HTTP_STATUS:"
  response=$(curl -sS -X POST "$url" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ${api_key}" \
    -d "$payload" \
    -w "\n${marker}%{http_code}")

  http_code="${response##*${marker}}"
  body="${response%$'\n'${marker}*}"

  if [[ "$http_code" != 2* ]]; then
    _aicli_err "❌ API 调用失败 (HTTP ${http_code})"
    _aicli_err "$body"
    return 1
  fi

  local api_error
  api_error=$(printf '%s' "$body" | jq -r '.error.message // empty' 2>/dev/null || true)
  if [[ -n "$api_error" ]]; then
    _aicli_err "❌ API 调用失败"
    _aicli_err "$body"
    return 1
  fi

  local content
  content=$(printf '%s' "$body" | jq -r '.choices[0].message.content // empty' 2>/dev/null || true)
  if [[ -z "$content" ]]; then
    _aicli_err "❌ API 返回解析为空"
    _aicli_err "$body"
    return 1
  fi

  printf '%s' "$content"
}

aicli_chat() {
  _aicli_check_deps
  local model="$1"
  local system_prompt="$2"
  local user_prompt="$3"
  local temperature="${4:-}"

  local messages_json
  messages_json=$(jq -cn \
    --arg system "$system_prompt" \
    --arg user "$user_prompt" \
    '[
      {role: "system", content: $system},
      {role: "user", content: $user}
    ]')

  aicli_chat_messages "$model" "$messages_json" "$temperature"
}
