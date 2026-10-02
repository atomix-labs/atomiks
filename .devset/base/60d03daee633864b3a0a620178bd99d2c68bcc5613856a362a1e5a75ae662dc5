# Checks commits against Conventional Commits: those a branch adds, or those a push landed.
check-git-commits:
    #!/usr/bin/env bash
    set -euo pipefail
    git rev-parse -q --verify HEAD > /dev/null || exit 0
    case ${GITHUB_EVENT_NAME:-} in
        # A pull request lands squashed, as one commit its title names, which the workflow `title`
        # checks: its own commits never land.
        pull_request | merge_group) exit 0 ;;
        # What a push to the default branch landed, a squash message edited at the merge included.
        push)
            before=$(jq -r '.before // empty' "$GITHUB_EVENT_PATH")
            if git rev-parse -q --verify "$before^{commit}" > /dev/null; then
                committed "$before..HEAD"
            fi
            exit 0
            ;;
    esac
    base=$(git rev-parse -q --verify "origin/${GITHUB_BASE_REF:-HEAD}" || true)
    [[ -n $base ]] || exit 0
    range="$(git merge-base "$base" HEAD)..HEAD"
    [[ -n $(git rev-list "$range") ]] || exit 0
    committed "$range"
