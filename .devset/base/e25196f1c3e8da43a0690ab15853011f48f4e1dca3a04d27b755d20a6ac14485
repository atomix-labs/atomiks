# Rules for GitHub Actions workflows, beside what actionlint and zizmor check. Input is
# every workflow and action together, as `conftest test --combine` gives it.
package workflows

workflows := [file |
	some file in input
	contains(file.path, ".github/workflows/")
]

# A job states how long it may run: a hung one otherwise holds a runner for six hours.
deny contains msg if {
	some file in workflows
	some name, job in file.contents.jobs
	not job["timeout-minutes"]
	not job.uses
	msg := sprintf("%s: job `%s` sets no timeout-minutes", [file.path, name])
}

# A workflow states what its token may do; the repository default grants more than any job needs.
deny contains msg if {
	some file in workflows
	not file.contents.permissions
	msg := sprintf("%s: states no permissions", [file.path])
}

# `needs.<job>.result` is the job's own outcome; an output named `result` reads as it, and is not.
deny contains msg if {
	some file in workflows
	some name, job in file.contents.jobs
	job.outputs.result
	msg := sprintf("%s: job `%s` names an output `result`, which `needs.%s.result` never reads", [file.path, name, name])
}

deny contains msg if {
	some file in input
	some name, job in file.contents.jobs
	some step in job.steps
	contains(step.with.script, "setOutput('result'")
	msg := sprintf("%s: job `%s` sets a step output `result`, which reads as the step's own outcome", [file.path, name])
}
