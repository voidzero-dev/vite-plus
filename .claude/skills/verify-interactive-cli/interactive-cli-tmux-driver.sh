#!/bin/bash
# Drive and capture an interactive `vp` (clack) prompt flow inside tmux for
# manual verification against an installed CLI.
#
# Usage:
#   interactive-cli-tmux-driver.sh <project-dir> "<command>" [STOP_AT_REGEX]
#     interactive-cli-tmux-driver.sh /tmp/demo "vp migrate"
#     interactive-cli-tmux-driver.sh /tmp/demo "vp migrate" "Upgrade Node.js"
#
# No STOP_AT  : run to completion, auto-accepting every prompt's DEFAULT (Enter),
#               then print a clean transcript.
# With STOP_AT: drive prompts until one matching the regex appears, then STOP
#               (do NOT answer it) and capture the pane twice 3s apart. Identical
#               captures => prompt is static (good). Differing captures (or a
#               "Checking ... (Xs)" line) => a spinner is animating UNDER the
#               prompt -- a real UX bug (this is how the Node-upgrade confirm
#               spinner-overlap was found).
#
# Why tmux and not `expect`: expect's raw capture is full of ANSI cursor/redraw
# noise; `tmux capture-pane -p` returns clean text because in-place redraws
# overwrite and only the resolved lines remain in scrollback.
# macOS has no tmux/timeout by default: `brew install tmux`.
set -u
DIR="${1:?project dir}"; CMD="${2:?command, e.g. \"vp migrate\"}"; STOP_AT="${3:-}"
command -v tmux >/dev/null || { echo "need tmux: brew install tmux" >&2; exit 1; }
S="clicap_$$"
cap1="$(mktemp)"; cap2="$(mktemp)"

tmux kill-session -t "$S" 2>/dev/null
tmux new-session -d -s "$S" -x 100 -y 50
tmux set-option -t "$S" history-limit 50000
# M1/M2 are split so the end marker never appears in the TYPED command line --
# otherwise a grep for it matches the echoed command, not the program output.
tmux send-keys -t "$S" 'export PS1="$ " PROMPT="%% " M1=CLI M2=CAPDONE' Enter; sleep 1
tmux send-keys -t "$S" "cd '$DIR'" Enter; sleep 1
tmux send-keys -t "$S" 'clear' Enter; sleep 1
tmux send-keys -t "$S" "$CMD"'; echo "$M1$M2 exit=$?"' Enter

prev=""; stable=0; sent=0
for i in $(seq 1 180); do
  sleep 2
  pane="$(tmux capture-pane -t "$S" -p -S -120 2>/dev/null)"
  # reached the prompt we want to inspect -> stop without answering it
  if [ -n "$STOP_AT" ] && printf '%s' "$pane" | grep -q "$STOP_AT"; then
    tmux capture-pane -t "$S" -p -S -60 > "$cap1"; sleep 3
    tmux capture-pane -t "$S" -p -S -60 > "$cap2"
    if diff -q "$cap1" "$cap2" >/dev/null; then
      echo "STATIC prompt (nothing animating underneath) -- OK"
    else
      echo "ANIMATING under the prompt (likely spinner-over-prompt bug):"
      diff "$cap1" "$cap2"
    fi
    echo "--- prompt ---"; sed -n "/$STOP_AT/,\$p" "$cap1"
    tmux kill-session -t "$S" 2>/dev/null; rm -f "$cap1" "$cap2"; exit 0
  fi
  # program finished
  printf '%s' "$pane" | grep -q "CLICAPDONE exit=" && break
  # A waiting prompt makes the pane STABLE; animating spinners keep it changing,
  # so this won't fire mid-work. Accept the default with Enter, once per state.
  if [ "$pane" = "$prev" ]; then
    stable=$((stable+1))
    if [ "$stable" -ge 2 ] && [ "$sent" -eq 0 ]; then tmux send-keys -t "$S" Enter; sent=1; fi
  else
    prev="$pane"; stable=0; sent=0
  fi
done

echo "=== clean transcript ==="
tmux capture-pane -t "$S" -p -S -3000
tmux kill-session -t "$S" 2>/dev/null; rm -f "$cap1" "$cap2"
