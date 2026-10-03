# A finding is fixed or allowed by hand: `typos --write-changes` also rewrites a misspelling a test
# means, so no fix recipe runs it.

# Checks the spelling of every file the repository tracks.
check-spelling:
    typos
