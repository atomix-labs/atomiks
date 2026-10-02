package workflows

good := {"path": ".github/workflows/ok.yml", "contents": {
	"permissions": {"contents": "read"},
	"jobs": {"build": {"timeout-minutes": 10, "steps": [{"run": "just check"}]}},
}}

test_a_workflow_that_keeps_the_rules_passes if {
	count(deny) == 0 with input as [good]
}

test_a_job_without_a_timeout_is_refused if {
	bad := json.patch(good, [{"op": "remove", "path": "/contents/jobs/build/timeout-minutes"}])
	deny[".github/workflows/ok.yml: job `build` sets no timeout-minutes"] with input as [bad]
}

test_a_reusable_workflow_call_needs_no_timeout if {
	call := json.patch(good, [{"op": "replace", "path": "/contents/jobs/build", "value": {"uses": "./.github/workflows/x.yml"}}])
	count(deny) == 0 with input as [call]
}

test_a_workflow_without_permissions_is_refused if {
	bad := json.patch(good, [{"op": "remove", "path": "/contents/permissions"}])
	deny[".github/workflows/ok.yml: states no permissions"] with input as [bad]
}

test_an_output_named_result_is_refused if {
	bad := json.patch(good, [{"op": "add", "path": "/contents/jobs/build/outputs", "value": {"result": "x"}}])
	count(deny) == 1 with input as [bad]
}

test_a_script_setting_result_is_refused if {
	step := {"uses": "actions/github-script@v9", "with": {"script": "core.setOutput('result', 1)"}}
	bad := json.patch(good, [{"op": "add", "path": "/contents/jobs/build/steps/-", "value": step}])
	count(deny) == 1 with input as [bad]
}
