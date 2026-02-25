#!/usr/bin/env bash
# test-all-profiles.sh — 测试所有 claudex profile，验证各模型身份
# 用法:
#   ./scripts/test-all-profiles.sh                            # 测试所有
#   ./scripts/test-all-profiles.sh openrouter-claude minimax  # 测试指定
#   NO_DEBUG=1 ./scripts/test-all-profiles.sh                 # 关闭 debug
#   TIMEOUT=600 ./scripts/test-all-profiles.sh                # 自定义超时
set -uo pipefail

PROMPT="What model are you? What company made you? Reply with model name and company only."
CLAUDEX="${CLAUDEX_BIN:-claudex}"
TIMEOUT="${TIMEOUT:-300}"
DEBUG="${DEBUG:-1}"
PASS=0
FAIL=0
SKIP=0
TOTAL=0
LOG_DIR="/tmp/claudex-test-$(date +%Y%m%d-%H%M%S)"

# 颜色
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
DIM='\033[2m'
BOLD='\033[1m'
NC='\033[0m'

ALL_PROFILES=(
  "openrouter-claude"
  "openrouter-gpt"
  "openrouter-gemini"
  "openrouter-deepseek"
  "openrouter-grok"
  "openrouter-qwen"
  "openrouter-llama"
  "minimax"
  "codex-sub"
)

# 支持命令行指定 profile
if [[ $# -gt 0 ]]; then
  PROFILES=("$@")
else
  PROFILES=("${ALL_PROFILES[@]}")
fi

# NO_DEBUG=1 关闭
[[ -n "${NO_DEBUG:-}" ]] && DEBUG=0

# 检查是否在 Claude Code 会话内
if [[ -n "${CLAUDECODE:-}" ]]; then
  unset CLAUDECODE
fi

# 检查依赖
for cmd in "$CLAUDEX" claude; do
  if ! command -v "$cmd" &>/dev/null; then
    echo -e "${RED}Error: '$cmd' not found.${NC}"
    exit 1
  fi
done

mkdir -p "$LOG_DIR"

echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  Claudex Profile Test Suite${NC}"
echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
echo -e "  Profiles:  ${#PROFILES[@]}"
echo -e "  Timeout:   ${TIMEOUT}s"
echo -e "  Debug:     $([ "$DEBUG" = "1" ] && echo "on → $LOG_DIR" || echo "off")"
echo -e "  Claudex:   $(command -v $CLAUDEX)"
echo -e "  Claude:    $(claude --version 2>/dev/null | head -1)"
echo ""

# 确保 proxy 先启动
echo -e "  ${DIM}\$ $CLAUDEX proxy start${NC}"
$CLAUDEX proxy start &>/dev/null &
sleep 2
echo ""

touch "$LOG_DIR/.start-marker"
RESULTS=()

for profile in "${PROFILES[@]}"; do
  ((TOTAL++))
  echo -e "${BOLD}──── [${TOTAL}/${#PROFILES[@]}] ${CYAN}${profile}${NC} ${BOLD}────${NC}"

  # 连通性测试
  echo -e "  ${DIM}\$ $CLAUDEX profile test $profile${NC}"
  CONN_OUTPUT=$($CLAUDEX profile test "$profile" 2>&1) || true
  if echo "$CONN_OUTPUT" | grep -q "FAIL"; then
    echo -e "  ${YELLOW}SKIP${NC} — $CONN_OUTPUT"
    ((SKIP++))
    RESULTS+=("${YELLOW}SKIP${NC}  $profile")
    echo ""
    continue
  fi
  echo -e "  ${DIM}${CONN_OUTPUT}${NC}"

  # 构造 claude 参数
  CLAUDE_ARGS=(
    -p "$PROMPT"
    --dangerously-skip-permissions
    --no-session-persistence
    --no-chrome
    --disable-slash-commands
    --tools ""
    --output-format text
  )

  PROFILE_LOG="$LOG_DIR/${profile}.log"

  if [[ "$DEBUG" = "1" ]]; then
    CLAUDE_ARGS+=(--debug-file "$PROFILE_LOG")
  fi

  # 运行测试
  echo -e "  ${DIM}\$ $CLAUDEX run $profile -p \"<prompt>\" \\\\${NC}"
  echo -e "  ${DIM}    --dangerously-skip-permissions --no-session-persistence \\\\${NC}"
  echo -e "  ${DIM}    --no-chrome --disable-slash-commands --tools \"\" \\\\${NC}"
  if [[ "$DEBUG" = "1" ]]; then
    echo -e "  ${DIM}    --debug-file ${PROFILE_LOG} \\\\${NC}"
  fi
  echo -e "  ${DIM}    --output-format text  ${YELLOW}(timeout: ${TIMEOUT}s)${NC}"
  START_TIME=$(date +%s)
  OUTPUT=""
  EXIT_CODE=0
  OUTPUT=$(timeout "$TIMEOUT" $CLAUDEX run "$profile" "${CLAUDE_ARGS[@]}" 2>"$LOG_DIR/${profile}.stderr") || EXIT_CODE=$?
  END_TIME=$(date +%s)
  DURATION=$((END_TIME - START_TIME))

  if [[ $EXIT_CODE -eq 0 && -n "$OUTPUT" ]]; then
    CLEAN_OUTPUT=$(echo "$OUTPUT" | grep -v '^\s*$' | grep -v '^[▐▝▘ █░▓╌─✳✻✽✶·⎿⏵❯]' | grep -v 'Claude Code' | grep -v '/model' | grep -v 'Claudex proxy' | grep -v 'bypass permissions' | grep -v 'Interrupted' | grep -v 'Resume this session' | tail -5)
    if [[ -n "$CLEAN_OUTPUT" ]]; then
      echo -e "  ${GREEN}PASS${NC} (${DURATION}s)"
      echo -e "  ${BOLD}→ ${CLEAN_OUTPUT}${NC}"
      ((PASS++))
      RESULTS+=("${GREEN}PASS${NC}  $profile (${DURATION}s)")
    else
      echo -e "  ${RED}FAIL${NC} (${DURATION}s) — empty after filtering"
      echo -e "  ${DIM}Raw (200 chars): ${OUTPUT:0:200}${NC}"
      ((FAIL++))
      RESULTS+=("${RED}FAIL${NC}  $profile — empty response")
    fi
  elif [[ $EXIT_CODE -eq 124 ]]; then
    echo -e "  ${RED}FAIL${NC} — timeout (${TIMEOUT}s)"
    ((FAIL++))
    RESULTS+=("${RED}FAIL${NC}  $profile — timeout")
  else
    echo -e "  ${RED}FAIL${NC} (${DURATION}s) — exit $EXIT_CODE"
    if [[ -n "$OUTPUT" ]]; then
      echo "$OUTPUT" | grep -v '^\s*$' | tail -3 | while IFS= read -r line; do
        echo -e "  ${DIM}${line}${NC}"
      done
    fi
    # stderr 也显示
    if [[ -s "$LOG_DIR/${profile}.stderr" ]]; then
      echo -e "  ${DIM}--- stderr ---${NC}"
      tail -3 "$LOG_DIR/${profile}.stderr" | while IFS= read -r line; do
        echo -e "  ${DIM}${line}${NC}"
      done
    fi
    ((FAIL++))
    RESULTS+=("${RED}FAIL${NC}  $profile — exit $EXIT_CODE")
  fi

  # proxy 日志摘要
  LATEST_PROXY=$(find ~/Library/Caches/claudex/ -name 'proxy-*.log' -newer "$LOG_DIR/.start-marker" 2>/dev/null | sort -r | head -1 || true)
  if [[ -n "$LATEST_PROXY" && -s "$LATEST_PROXY" ]]; then
    UPSTREAM=$(grep "upstream response\|upstream error" "$LATEST_PROXY" 2>/dev/null | tail -1 | sed 's/.*INFO //' || true)
    [[ -n "$UPSTREAM" ]] && echo -e "  ${DIM}Proxy: ${UPSTREAM}${NC}"
  fi

  echo ""
done

# 汇总
echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  Summary${NC}"
echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
for r in "${RESULTS[@]}"; do
  echo -e "  $r"
done
echo ""
echo -e "  ${GREEN}${PASS} passed${NC} / ${RED}${FAIL} failed${NC} / ${YELLOW}${SKIP} skipped${NC} / ${TOTAL} total"
echo ""
echo -e "  ${DIM}Debug logs: $LOG_DIR/${NC}"
LATEST_PROXY=$(ls -t ~/Library/Caches/claudex/proxy-*.log 2>/dev/null | head -1 || true)
[[ -n "$LATEST_PROXY" ]] && echo -e "  ${DIM}Proxy log:  $LATEST_PROXY${NC}"
echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"

exit 0
