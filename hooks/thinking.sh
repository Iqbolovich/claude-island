#!/usr/bin/env bash
# Хук Claude Code для Claude Island.
#   on   — UserPromptSubmit / PostToolUse: Claude работает (метка в thinking/)
#   off  — Stop / SessionEnd: Claude закончил
#   wait — Notification: если Claude просит разрешения, метка в waiting/
base="${APPDATA:-$HOME/AppData/Roaming}/com.oligarh.claudeisland"
in=$(head -c 8000)
id=$(printf '%s' "$in" | grep -o '"session_id" *: *"[^"]*"' | head -1 | sed 's/.*"\([^"]*\)"$/\1/')
id="${id:-default}"
mkdir -p "$base/thinking" "$base/waiting"
case "$1" in
  on)   touch "$base/thinking/$id"; rm -f "$base/waiting/$id" ;;
  wait) printf '%s' "$in" | grep -qi 'permission' && touch "$base/waiting/$id" ;;
  *)    rm -f "$base/thinking/$id" "$base/waiting/$id" ;;
esac
exit 0
