#!/usr/bin/env bash
set -euo pipefail

DEST_DIR="${1:-${XDG_BIN_HOME:-$HOME/.local/bin}}"
BIN_PATH="${DEST_DIR}/hugdocker"
LEGACY_BIN_PATH="${DEST_DIR}/dockerctl"

if [[ ! -e "${BIN_PATH}" && ! -e "${LEGACY_BIN_PATH}" ]]; then
  echo "未找到安装文件: ${BIN_PATH}"
  exit 0
fi

rm -f "${BIN_PATH}" "${LEGACY_BIN_PATH}"
echo "已卸载: ${BIN_PATH}"
