#!/bin/zsh

set -euo pipefail

ROOT_DIR="${0:A:h}"
CONFIG_DIR="${HOME}/.config/aiclikits"
CONFIG_FILE="${CONFIG_DIR}/config.json"
TARGET_BIN_DIR="${HOME}/.local/bin"

if [[ ! -d "$TARGET_BIN_DIR" ]]; then
  mkdir -p "$TARGET_BIN_DIR"
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "❌ 未找到 cargo，请先安装 Rust 环境"
  exit 1
fi

echo "📦 正在编译 Rust 发布版本..."
cd "$ROOT_DIR"
cargo build --release

BUILD_BIN="${ROOT_DIR}/target/release/aid"

if [[ ! -f "$BUILD_BIN" ]]; then
  echo "❌ 编译失败，未找二进制文件: $BUILD_BIN"
  exit 1
fi

install -m 0755 "$BUILD_BIN" "${TARGET_BIN_DIR}/aid"

cd "$TARGET_BIN_DIR"
ln -sf aid aiq
ln -sf aid aiask
ln -sf aid aish
ln -sf aid aitrans
ln -sf aid ait

mkdir -p "$CONFIG_DIR"
if [[ ! -f "$CONFIG_FILE" ]]; then
  cp "${ROOT_DIR}/config.example.json" "$CONFIG_FILE"
  echo "✅ 已创建默认配置: $CONFIG_FILE"
else
  echo "ℹ️ 已存在配置: $CONFIG_FILE（未覆盖）"
fi

echo ""
echo "✅ Rust aid 安装完成！已安装到 ${TARGET_BIN_DIR}/aid 及别名符号链接"
echo ""
echo "配置文件位置："
echo "  $CONFIG_FILE"
echo ""
echo "请确保 $TARGET_BIN_DIR 在 PATH 中："
echo "  export PATH=\"$TARGET_BIN_DIR:\$PATH\""
