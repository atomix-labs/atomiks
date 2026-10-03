#!/usr/bin/env bash
# Claude Code's hooks: a turn that changes the repository ends once `just check` passes. On each
# prompt, `check.sh start` notes the repository's state; when the agent stops, `check.sh` runs
# `just check` only if that state has changed, so a turn that reads, plans or answers ends at once.
# The check runs through mise, as CI's does, since a hook's shell has no mise activated. Its
# failures block the stop, and the agent works on; if it then changes nothing, it may stop, so a
# failure it cannot fix never loops.
set -uo pipefail

input=$(cat)
cd "${CLAUDE_PROJECT_DIR:-.}" 2> /dev/null && git rev-parse --git-dir > /dev/null 2>&1 || exit 0

# The repository's state: HEAD, the changes to tracked files, and every untracked file.
state() {
    local files
    {
        git rev-parse HEAD
        git diff HEAD --binary
        files=$(git ls-files --others --exclude-standard)
        [[ -z $files ]] || {
            printf '%s\n' "$files"
            git hash-object --stdin-paths <<< "$files"
        }
    } 2> /dev/null | git hash-object --stdin
}

# Prints the JSON object Claude Code reads from a hook, from each key and its text.
reply() {
    local separator="" text
    printf '{'
    while (($# > 1)); do
        text=$(printf '%s' "$2" | sed $'s/\e\\[[0-9;]*[A-Za-z]//g' | tr -d '\000-\010\013-\037')
        text=${text//\\/\\\\}
        text=${text//\"/\\\"}
        text=${text//$'\t'/\\t}
        text=${text//$'\n'/\\n}
        printf '%s"%s": "%s"' "$separator" "$1" "$text"
        separator=", "
        shift 2
    done
    printf '}\n'
}

session=$(tr -d '\n' <<< "$input" | sed -n 's/.*"session_id" *: *"\([A-Za-z0-9_-]*\)".*/\1/p')
dir=$(git rev-parse --git-path claude-hook)
mkdir -p "$dir" || exit 0
start=$dir/${session:-session}.start blocked=$dir/${session:-session}.blocked
now=$(state)

if [[ ${1:-} == start ]]; then
    find "$dir" -type f -mtime +7 -delete 2> /dev/null
    printf '%s\n' "$now" > "$start"
    rm -f "$blocked"
    exit 0
fi
# Nothing changed this turn, or since the stop last blocked.
[[ -f $start && $(< "$start") != "$now" ]] || exit 0
[[ -f $blocked && $(< "$blocked") == "$now" ]] && exit 0

# What keeps the check from running is the user's to fix, not the agent's: they are told, and the
# agent stops.
skip() {
    reply systemMessage "The Stop hook skipped \`just check\`: $1"
    exit 0
}
just=(just)
mise=$(command -v mise || printf '%s' "$HOME/.local/bin/mise")
if [[ -x $mise ]]; then
    PATH=${mise%/*}:$PATH
    just=("$mise" exec -- just)
    error=$("${just[@]}" --version 2>&1 > /dev/null) || skip "$(head -n 20 <<< "$error")"
elif [[ -e mise.toml || -e .mise.toml || -e .config/mise.toml || -d .config/mise ]]; then
    skip "mise, which provides the repository's tools, is not installed."
elif ! command -v just > /dev/null; then
    skip "just is not installed."
fi

if output=$("${just[@]}" check 2>&1); then
    rm -f "$blocked"
    exit 0
fi
printf '%s\n' "$now" > "$blocked"
reply decision block reason "\`just check\` fails after this turn's changes. Fix what they broke, then stop; if a failure was there before them, or is not yours to fix, say so and stop.

$(tail -n 40 <<< "$output")"
