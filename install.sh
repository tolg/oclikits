#!/bin/zsh

set -euo pipefail

ROOT_DIR="${0:A:h}"
BIN_SRC="${ROOT_DIR}/bin"
CONFIG_DIR="${HOME}/.config/aiclikits"
CONFIG_FILE="${CONFIG_DIR}/config.json"

TARGET_BIN_DIR="${HOME}/.local/bin"
TARGET_LIB_DIR="${HOME}/.local/lib/aiclikits"
TARGET_LIB_FILE="${TARGET_LIB_DIR}/aicli.sh"
if [[ ! -d "$TARGET_BIN_DIR" ]]; then
  mkdir -p "$TARGET_BIN_DIR"
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "❌ 未安装 jq，请先安装: brew install jq"
  exit 1
fi
if ! command -v curl >/dev/null 2>&1; then
  echo "❌ 未安装 curl"
  exit 1
fi

install -m 0755 "${BIN_SRC}/aish" "${TARGET_BIN_DIR}/aish"
install -m 0755 "${BIN_SRC}/aiq" "${TARGET_BIN_DIR}/aiq"
install -m 0755 "${BIN_SRC}/aiask" "${TARGET_BIN_DIR}/aiask"
install -m 0755 "${BIN_SRC}/aitrans" "${TARGET_BIN_DIR}/aitrans"
install -m 0755 "${BIN_SRC}/ait" "${TARGET_BIN_DIR}/ait"

mkdir -p "$TARGET_LIB_DIR"
install -m 0644 "${ROOT_DIR}/lib/aicli.sh" "$TARGET_LIB_FILE"

mkdir -p "$CONFIG_DIR"
if [[ ! -f "$CONFIG_FILE" ]]; then
  cp "${ROOT_DIR}/config.example.json" "$CONFIG_FILE"
  echo "✅ 已创建默认配置: $CONFIG_FILE"
else
  echo "ℹ️ 已存在配置: $CONFIG_FILE（未覆盖）"
fi

echo ""
echo "✅ 安装完成"
echo ""
echo "下一步：编辑应用配置文件："
echo "  $CONFIG_FILE"
echo ""
echo "至少设置："
echo "  .openai.apiKey"
echo "可选设置："
echo "  .openai.baseUrl"
echo ""
echo "并确保 $TARGET_BIN_DIR 在 PATH 中："
echo "  export PATH=\"$TARGET_BIN_DIR:$PATH\""
