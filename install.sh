#!/bin/zsh

set -euo pipefail

ROOT_DIR="${0:A:h}"
TARGET_BIN_DIR="${HOME}/.local/bin"
BINARY_NAME="oc"
ALIASES=(ocq ocsq oct ocsh occmt octrans aiq aiask aish ait aitrans)

# Detect environment language
LANG_ENV="${OCLIKITS_LANG:-${AID_LANG:-${LC_ALL:-${LC_MESSAGES:-${LANG:-}}}}}"
if [[ "${LANG_ENV:l}" == *zh* ]]; then
  IS_ZH=1
else
  IS_ZH=0
fi

say() {
  if [[ $IS_ZH -eq 1 ]]; then
    echo "$1"
  else
    echo "$2"
  fi
}

if [[ ! -d "$TARGET_BIN_DIR" ]]; then
  mkdir -p "$TARGET_BIN_DIR"
fi

if ! command -v cargo >/dev/null 2>&1; then
  say "❌ 未找到 cargo，请先安装 Rust 环境" \
    "❌ cargo not found. Please install the Rust toolchain first."
  exit 1
fi

say "📦 正在编译 Rust 发布版本..." \
  "📦 Building the Rust release binary..."

cd "$ROOT_DIR"
cargo build --release

BUILD_BIN="${ROOT_DIR}/target/release/${BINARY_NAME}"

if [[ ! -f "$BUILD_BIN" ]]; then
  say "❌ 编译失败，未找到二进制文件: $BUILD_BIN" \
    "❌ Build failed, binary not found: $BUILD_BIN"
  exit 1
fi

install -m 0755 "$BUILD_BIN" "${TARGET_BIN_DIR}/${BINARY_NAME}"

# Remove the old `aiclik` binary/symlink left by previous installs.
if [[ -e "${TARGET_BIN_DIR}/aiclik" || -L "${TARGET_BIN_DIR}/aiclik" ]]; then
  rm -f "${TARGET_BIN_DIR}/aiclik"
  say "🧹 已移除旧命令: ${TARGET_BIN_DIR}/aiclik" \
    "🧹 Removed legacy binary: ${TARGET_BIN_DIR}/aiclik"
fi

cd "$TARGET_BIN_DIR"
for alias in "${ALIASES[@]}"; do
  ln -sf "$BINARY_NAME" "$alias"
done

# Create the model config template (~/.config/oclikits/config.jsonc) if missing.
# Existing config files are never overwritten.
"${TARGET_BIN_DIR}/${BINARY_NAME}" init-config

if ! command -v opencode >/dev/null 2>&1; then
  say "⚠️ 未找到 opencode，命令需要 OpenCode v2 才能运行: https://opencode.ai" \
    "⚠️ opencode not found. Commands require OpenCode v2: https://opencode.ai"
fi

echo ""
if [[ $IS_ZH -eq 1 ]]; then
  echo "✅ oclikits 安装完成！已安装 ${TARGET_BIN_DIR}/${BINARY_NAME} 及命令别名:"
  echo "  ${ALIASES[*]}"
  echo ""
  echo "会话文件位置（自动创建）："
  echo "  ~/.config/oclikits/sessions.json"
  echo ""
  echo "请编辑模型配置，至少填写 model："
  echo "  ~/.config/oclikits/config.jsonc"
  echo ""
  echo "请确保 $TARGET_BIN_DIR 在 PATH 中："
  echo "  export PATH=\"$TARGET_BIN_DIR:\$PATH\""
else
  echo "✅ oclikits installation complete! Installed ${TARGET_BIN_DIR}/${BINARY_NAME} with aliases:"
  echo "  ${ALIASES[*]}"
  echo ""
  echo "Session file (created automatically):"
  echo "  ~/.config/oclikits/sessions.json"
  echo ""
  echo "Edit the model config and fill in at least model:"
  echo "  ~/.config/oclikits/config.jsonc"
  echo ""
  echo "Please ensure $TARGET_BIN_DIR is in PATH:"
  echo "  export PATH=\"$TARGET_BIN_DIR:\$PATH\""
fi
