#!/usr/bin/env bash
# Readies a machine to develop a repository, cloning it when run elsewhere: mise, pinned and checked
# against its sha256, then `mise bootstrap`, which installs every tool the lock pins and runs `just
# setup`. Run it again at any time: what is in place is kept. `setup.sh --help` says how.
set -euo pipefail

# The mise every machine bootstraps with, and the sha256 of its release for each platform.
MISE_VERSION=2026.9.13
MISE_SHA256_LINUX_X64=03052ce8247285b8b5ec40dd04f8058dabc1eaa29881550b7d21199cb133fbf5
MISE_SHA256_LINUX_ARM64=9a6c72ee899c1e95988e6eb668a9860c4f8e3c01bc2aedd9de113d58b1e35869
MISE_SHA256_MACOS_ARM64=4698c2537eef78830bd5acf98204100fb0ad9a8884861e55265e855f35aa3fa0

repo="" dir="" host=false activate=false yes=false dry_run=false mise=""

say() { printf 'setup: %s\n' "$*" >&2; }

die() {
    say "$*"
    exit 1
}

usage() {
    cat << 'EOF'
Readies a machine to develop a repository, cloning it when run elsewhere.

  curl -fsSL https://atomix-labs.github.io/atxp/setup.sh | bash -s -- <repo>
  setup.sh [<repo>] [--dir <dir>] [--host] [--activate] [--yes] [--dry-run]

  <repo>      host/owner/name, over SSH when the host takes your key and over HTTPS with gh's
              login otherwise (SETUP_PROTOCOL=ssh or https decides instead), or any git URL;
              none inside a checkout
  --dir       where to clone: ./<name> unless given
  --host      then `just host`: the machine's own setup, whose steps may ask for sudo
  --activate  add mise's activation to your shell's rc, through mise
  --yes       ask nothing, as under CI
  --dry-run   say what would happen, and change nothing
EOF
}

parse() {
    while (($#)); do
        case $1 in
            --dir)
                (($# > 1)) || die "--dir needs a directory"
                dir=$2
                shift
                ;;
            --host) host=true ;;
            --activate) activate=true ;;
            -y | --yes) yes=true ;;
            -n | --dry-run) dry_run=true ;;
            -h | --help)
                usage
                exit 0
                ;;
            -*) die "unknown option $1: see --help" ;;
            *)
                [[ -z $repo ]] || die "one repository at a time: $repo, $1"
                repo=$1
                ;;
        esac
        shift
    done
    if [[ -n ${CI:-} ]]; then yes=true; fi
}

# Whether a person can answer: not --yes, not CI, and a terminal to ask on.
interactive() {
    ! $yes && { : < /dev/tty; } 2> /dev/null
}

# Whether version $1 is MISE_VERSION or later.
at_least() {
    local IFS=. have want i
    read -ra have <<< "$1"
    read -ra want <<< "$MISE_VERSION"
    for i in 0 1 2; do
        ((${have[i]:-0} > ${want[i]:-0})) && return 0
        ((${have[i]:-0} < ${want[i]:-0})) && return 1
    done
    return 0
}

# A mise of MISE_VERSION or later: the one on PATH, or ~/.local/bin/mise.
find_mise() {
    local candidate version
    for candidate in "$(command -v mise || true)" "$HOME/.local/bin/mise"; do
        [[ -n $candidate && -x $candidate ]] || continue
        version=$("$candidate" version 2> /dev/null | cut -d' ' -f1) || continue
        if at_least "$version"; then
            echo "$candidate"
            return 0
        fi
    done
    return 1
}

# This machine's mise release and its sha256; a platform the locks do not hold is refused.
release() {
    case "$(uname -s)-$(uname -m)" in
        Linux-x86_64 | Linux-amd64) echo "linux-x64-musl $MISE_SHA256_LINUX_X64" ;;
        Linux-aarch64 | Linux-arm64) echo "linux-arm64-musl $MISE_SHA256_LINUX_ARM64" ;;
        Darwin-arm64) echo "macos-arm64 $MISE_SHA256_MACOS_ARM64" ;;
        *) die "$(uname -s) $(uname -m) is not supported: the locks hold linux-x64, linux-arm64 and macos-arm64" ;;
    esac
}

sha256() {
    if command -v sha256sum > /dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1
}

# Installs mise MISE_VERSION to ~/.local/bin, once its download matches the sha256 above.
install_mise() {
    local found asset sum file
    found=$(release)
    read -r asset sum <<< "$found"
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    file=$tmp/mise-v$MISE_VERSION-$asset.tar.gz
    say "installing mise $MISE_VERSION to ~/.local/bin"
    curl --proto '=https' --tlsv1.2 -fsSL -o "$file" \
        "https://github.com/jdx/mise/releases/download/v$MISE_VERSION/${file##*/}"
    [[ $(sha256 "$file") == "$sum" ]] || die "the mise download does not match its sha256: nothing installed"
    tar -xzf "$file" -C "$tmp"
    mkdir -p "$HOME/.local/bin"
    mv "$tmp/mise/bin/mise" "$HOME/.local/bin/mise"
}

# The SSH user on host $1: GitHub Enterprise's *.ghe.com hosts take their subdomain; others, git.
ssh_user() {
    case $1 in
        *.ghe.com) echo "${1%%.*}" ;;
        *) echo git ;;
    esac
}

# Whether host $1 takes this machine's SSH key.
ssh_works() {
    local out
    out=$(ssh -T -o BatchMode=yes -o ConnectTimeout=10 -o StrictHostKeyChecking=accept-new \
        "$(ssh_user "$1")@$1" 2>&1) || true
    [[ $out == *"successfully authenticated"* ]]
}

# The URL to clone $1 from: a git URL as it is; host/owner/name over SSH or HTTPS.
clone_url() {
    local host=${1%%/*} path=${1#*/}
    case $1 in
        *://* | *@*:*)
            echo "$1"
            return 0
            ;;
    esac
    [[ $host != "$1" && $path == */* ]] || die "$1 is neither a git URL nor host/owner/name"
    path=${path%.git}
    case ${SETUP_PROTOCOL:-} in
        ssh) echo "$(ssh_user "$host")@$host:$path.git" ;;
        https) echo "https://$host/$path.git" ;;
        "") if ssh_works "$host"; then echo "$(ssh_user "$host")@$host:$path.git"; else echo "https://$host/$path.git"; fi ;;
        *) die "SETUP_PROTOCOL is ssh or https, not $SETUP_PROTOCOL" ;;
    esac
}

# gh, at its newest release past the three-day cooldown: it only logs in.
run_gh() {
    MISE_MINIMUM_RELEASE_AGE=3d "$mise" exec gh@latest -- gh "$@"
}

# Makes sure git can read the HTTPS URL $1: as it is, or once gh has logged in to its host.
https_access() {
    local host
    [[ $1 == https://* ]] || return 0
    GIT_TERMINAL_PROMPT=0 git ls-remote "$1" > /dev/null 2>&1 && return 0
    host=${1#https://}
    host=${host%%/*}
    if [[ -z ${GH_TOKEN:-} ]]; then
        interactive || die "cannot read $1: add an SSH key to $host, or set GH_TOKEN"
        run_gh auth login --hostname "$host" --git-protocol https --web < /dev/tty
    fi
    run_gh auth setup-git --hostname "$host"
}

# Repository address $1, whatever its form, as host/owner/name in lower case.
slug() {
    local s=${1#*://}
    s=${s#*@}
    s=${s/:/\/}
    s=${s%/}
    s=${s%.git}
    printf '%s\n' "$s" | tr '[:upper:]' '[:lower:]'
}

# The root of the checkout this runs in, when it is one of the repository asked for.
checkout() {
    local root origin
    root=$(git rev-parse --show-toplevel 2> /dev/null) || return 1
    if [[ -n $repo ]]; then
        origin=$(git -C "$root" remote get-url origin 2> /dev/null) || return 1
        [[ $(slug "$origin") == "$(slug "$repo")" ]] || return 1
    fi
    echo "$root"
}

bootstrap() {
    local args=(bootstrap --locked)
    if $yes || ! interactive; then args+=(--yes); fi
    if $dry_run; then args+=(--dry-run); fi
    "$mise" "${args[@]}" "$@"
}

# The shell whose rc gains mise's activation: bash, zsh or fish.
user_shell() {
    case ${SHELL##*/} in
        zsh | fish) echo "${SHELL##*/}" ;;
        *) echo bash ;;
    esac
}

activation() {
    local shell
    shell=$(user_shell)
    if $activate; then
        mkdir -p "$HOME/.config/mise/conf.d"
        printf '[bootstrap.mise_shell_activate]\n%s = true\n' "$shell" \
            > "$HOME/.config/mise/conf.d/devset-activate.toml"
        "$mise" bootstrap mise-shell-activate apply --yes
    elif [[ -z ${MISE_SHELL:-} ]]; then
        say "to have the tools in every new shell, add this line to your $shell rc:"
        if [[ $shell == fish ]]; then say "    $mise activate fish | source"; else say "    eval \"\$($mise activate $shell)\""; fi
    fi
    [[ $(command -v mise || true) == "$mise" ]] || say "and put ${mise%/*} on PATH ahead of any other mise"
}

main() {
    parse "$@"
    command -v git > /dev/null || die "git is needed first"
    command -v curl > /dev/null || die "curl is needed first"
    if ! mise=$(find_mise); then
        if $dry_run; then
            say "would install mise $MISE_VERSION to ~/.local/bin, then bootstrap"
            return 0
        fi
        install_mise
        mise=$HOME/.local/bin/mise
    fi
    local root url
    if root=$(checkout); then
        bootstrap --cd "$root"
    else
        [[ -n $repo ]] || die "not in a checkout: name the repository to clone, as host/owner/name"
        url=$(clone_url "$repo")
        if ! $dry_run; then https_access "$url"; fi
        root=${dir:-$(basename "$(slug "$url")")}
        [[ $root == /* ]] || root=$PWD/$root
        if $dry_run; then say "would clone $url into $root"; else say "cloning $url into $root"; fi
        bootstrap --from "$url" --from-dir "$root"
    fi
    if $host; then
        if $dry_run; then say "would run just host in $root"; else (cd "$root" && "$mise" exec -- just host); fi
    fi
    if ! $dry_run; then activation; fi
}

main "$@"
