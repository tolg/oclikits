#!/bin/zsh

set -euo pipefail

ROOT_DIR="${0:A:h}"
CONFIG_DIR="${HOME}/.config/aiclikits"
CONFIG_FILE="${CONFIG_DIR}/config.json"
TARGET_BIN_DIR="${HOME}/.local/bin"

# Detect environment language
LANG_ENV="${AID_LANG:-${LC_ALL:-${LC_MESSAGES:-${LANG:-}}}}"
if [[ "$LANG_ENV" =~ "(?i)zh" ]]; then
  IS_ZH=1
else
  IS_ZH=0
fi

if [[ ! -d "$TARGET_BIN_DIR" ]]; then
  mkdir -p "$TARGET_BIN_DIR"
fi

if ! command -v cargo >/dev/null 2>&1; then
  if [[ $IS_ZH -eq 1 ]]; then
    echo "❌ 未找到 cargo，请先安装 Rust 环境"
  else
    echo "❌ cargo not found. Please install Rust environment first."
  fi
  exit 1
fi

if [[ $IS_ZH -eq 1 ]]; then
  echo "📦 正在编译 Rust 发布版本..."
else
  echo "📦 Building Rust release version..."
fi

cd "$ROOT_DIR"
cargo build --release

BUILD_BIN="${ROOT_DIR}/target/release/aiclik"

if [[ ! -f "$BUILD_BIN" ]]; then
  if [[ $IS_ZH -eq 1 ]]; then
    echo "❌ 编译失败，未找到二进制文件: $BUILD_BIN"
  else
    echo "❌ Build failed, binary file not found: $BUILD_BIN"
  fi
  exit 1
fi

install -m 0755 "$BUILD_BIN" "${TARGET_BIN_DIR}/aiclik"

cd "$TARGET_BIN_DIR"
ln -sf aiclik aiq
ln -sf aiclik aiask
ln -sf aiclik aish
ln -sf aiclik aitrans
ln -sf aiclik ait

mkdir -p "$CONFIG_DIR"
if [[ ! -f "$CONFIG_FILE" ]]; then
  cp "${ROOT_DIR}/config.example.json" "$CONFIG_FILE"
  if [[ $IS_ZH -eq 1 ]]; then
    echo "✅ 已创建默认配置: $CONFIG_FILE"
  else
    echo "✅ Created default config: $CONFIG_FILE"
  fi
else
  if [[ $IS_ZH -eq 1 ]]; then
    echo "ℹ️ 已存在配置: $CONFIG_FILE（未覆盖）"
  else
    echo "ℹ️ Existing config found: $CONFIG_FILE (not overwritten)"
  fi
fi

echo ""
if [[ $IS_ZH -eq 1 ]]; then
  echo "✅ Rust aiclik 安装完成！已安装到 ${TARGET_BIN_DIR}/aiclik 及别名符号链接"
  echo ""
  echo "配置文件位置："
  echo "  $CONFIG_FILE"
  echo ""
  echo "请确保 $TARGET_BIN_DIR 在 PATH 中："
  echo "  export PATH=\"$TARGET_BIN_DIR:\$PATH\""
else
  echo "✅ Rust aiclik installation complete! Installed to ${TARGET_BIN_DIR}/aiclik and alias symlinks"
  echo ""
  echo "Config file location:"
  echo "  $CONFIG_FILE"
  echo ""
  echo "Please ensure $TARGET_BIN_DIR is in PATH:"
  echo "  export PATH=\"$TARGET_BIN_DIR:\$PATH\""
fi
