# Checks every tool is locked for every platform, each download verified.
check-mise:
    mise exec -- python3 .just/mise.py check

# Moves the repository's own tools to their newest releases past the cooldown, and locks them.
bump-mise:
    mise exec -- python3 .just/mise.py bump "${BUMP_REPORT_DIR:+$BUMP_REPORT_DIR/mise.md}"
