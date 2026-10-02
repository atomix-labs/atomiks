# Fails when a file devset manages has drifted from its profile, or an update is unfinished.
check-devset:
    devset status --exit-code

# Moves each source the target names by tag to its newest release past the cooldown, and applies it.
bump-devset:
    mise exec -- python3 -B .just/devset.py bump "${BUMP_REPORT_DIR:+$BUMP_REPORT_DIR/devset.md}"
