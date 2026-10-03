# GitHub's limits: a name of at most 50 characters, a colour of six hex digits, a description of at
# most 100 characters.

# Checks .github/labels.toml, and that the forms, automation and Dependabot name only its labels.
check-github-labels:
    mise exec -- python3 -B .just/github-labels/labels.py check
