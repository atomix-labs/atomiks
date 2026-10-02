// Watches every scheduled workflow, read from .github/workflows/ itself, so none needs listing: one
// whose last run on the default branch failed, on schedule or by hand, or that has not run on
// schedule by twice the interval it keeps, or that GitHub has disabled, gets an issue until it
// recovers. A run by hand after a fix is a recovery, so the issue closes without waiting for the
// schedule.
const fs = require("fs");
const { hold, release } = require(`${process.env.GITHUB_WORKSPACE}/.github/scripts/automation.js`);

const FAILED = new Set(["failure", "timed_out", "startup_failure", "action_required"]);
const GRACE = 60 * 60 * 1000;

// The workflow files with a schedule among their triggers.
function scheduled() {
  const dir = `${process.env.GITHUB_WORKSPACE}/.github/workflows`;
  return fs.readdirSync(dir)
    .filter((name) => /\.ya?ml$/.test(name))
    .filter((name) => /^\s+schedule:/m.test(fs.readFileSync(`${dir}/${name}`, "utf8")))
    .map((name) => `.github/workflows/${name}`);
}

// The workflow's latest runs from `event`, newest first, on `branch` if given.
async function runs({ github, context }, workflow, event, branch) {
  const { data } = await github.rest.actions.listWorkflowRuns({
    ...context.repo,
    workflow_id: workflow.id,
    event,
    ...(branch ? { branch } : {}),
    per_page: 5,
  });
  return data.workflow_runs;
}

// Why the workflow is unwell, or null when it is well, or undefined when there is nothing to say.
async function verdict({ github, context }, workflow, branch) {
  if (workflow.state !== "active") return `is ${workflow.state.replaceAll("_", " ")}`;
  const scheduled = await runs({ github, context }, workflow, "schedule");
  if (scheduled.length >= 2) {
    const [last, before] = scheduled.map((run) => Date.parse(run.created_at));
    if (Date.now() - last > 2 * (last - before) + GRACE) {
      return `has not run on schedule since ${scheduled[0].created_at}`;
    }
  }
  const [done] = [
    ...scheduled,
    ...(await runs({ github, context }, workflow, "workflow_dispatch", branch)),
  ]
    .filter((run) => run.status === "completed")
    .sort((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at));
  if (!done) return undefined;
  const how = done.event === "schedule" ? "on schedule" : "by hand";
  if (FAILED.has(done.conclusion)) return `failed its last run, ${how}: ${done.html_url}`;
  return done.conclusion === "success" ? null : undefined;
}

module.exports = async ({ github, context, core }) => {
  const paths = new Set(scheduled());
  const { data: repository } = await github.rest.repos.get(context.repo);
  const workflows = await github.paginate(github.rest.actions.listRepoWorkflows, {
    ...context.repo,
    per_page: 100,
  });
  for (const workflow of workflows.filter((w) => paths.has(w.path))) {
    const title = `The ${workflow.name} workflow is failing`;
    const why = await verdict({ github, context }, workflow, repository.default_branch);
    if (why) {
      core.warning(`${workflow.name} ${why}`);
      await hold({ github, context }, "broken", title, `${workflow.name} ${why}.`);
    } else if (why === null) {
      await release({ github, context }, "broken", title, `Recovered at ${context.sha}.`);
    }
  }
};
