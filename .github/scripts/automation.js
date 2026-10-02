// The automation's issues and pull requests, labelled, typed and assigned as
// .github/automation.json says, which may leave any of them out.
//
// An issue is one per condition, found by its exact title: opened when the condition starts,
// commented on while it lasts, closed when it ends. A pull request is one per branch: opened, or
// updated, with its kind's labels and the assignees, who are told in a comment what it holds and
// what is theirs to do; where the repository turns merging on, it merges once its gate has passed.
const fs = require("fs");

const DEFAULTS = {
  broken: { labels: ["automation"] },
  chore: { labels: ["dependencies", "automation"] },
  docs: { labels: ["automation"] },
  assignees: [],
};

// The automation settings: the repository's own, over the defaults.
function settings() {
  const path = `${process.env.GITHUB_WORKSPACE}/.github/automation.json`;
  const own = fs.existsSync(path) ? JSON.parse(fs.readFileSync(path, "utf8")) : {};
  return { ...DEFAULTS, ...own };
}

// The assignees, as mentions to open a comment with; empty when there are none.
function mention() {
  const logins = settings().assignees ?? [];
  return logins.length ? `${logins.map((login) => `@${login}`).join(" ")}: ` : "";
}

// The open issue titled `title` among those labelled as `kind` says, if there is one.
async function find({ github, context }, kind, title) {
  const { labels } = settings()[kind] ?? DEFAULTS[kind];
  const issues = await github.paginate(github.rest.issues.listForRepo, {
    ...context.repo,
    state: "open",
    labels: labels.join(","),
    per_page: 100,
  });
  return issues.find((issue) => issue.title === title && !issue.pull_request);
}

// Opens the issue `title`, or comments on it if it is open and has not said `body` yet: the
// condition holds, and each new turn of it is told once.
async function hold({ github, context }, kind, title, body) {
  const open = await find({ github, context }, kind, title);
  if (open) {
    const comments = await github.paginate(github.rest.issues.listComments, {
      ...context.repo,
      issue_number: open.number,
      per_page: 100,
    });
    if (![open.body, ...comments.map((comment) => comment.body)].includes(body)) {
      await github.rest.issues.createComment({ ...context.repo, issue_number: open.number, body });
    }
    return open.number;
  }
  const config = settings();
  const { labels, type } = config[kind] ?? DEFAULTS[kind];
  // A raw request, so an issue type goes through where the REST client does not know the field.
  const { data } = await github.request("POST /repos/{owner}/{repo}/issues", {
    ...context.repo,
    title,
    body,
    labels,
    assignees: config.assignees ?? [],
    ...(type ? { type } : {}),
  });
  return data.number;
}

// Comments on the issue `title` and closes it, if it is open: the condition has ended.
async function release({ github, context }, kind, title, body) {
  const open = await find({ github, context }, kind, title);
  if (!open) return;
  await github.rest.issues.createComment({ ...context.repo, issue_number: open.number, body });
  await github.rest.issues.update({ ...context.repo, issue_number: open.number, state: "closed" });
}

// Opens the pull request from `branch`, or updates the open one, labelled as `kind` says and
// assigned; `note` is the comment that tells the assignees what changed.
async function propose({ github, context }, kind, { branch, title, body, note }) {
  const config = settings();
  const { labels } = config[kind] ?? DEFAULTS[kind];
  const open = await github.rest.pulls.list({
    ...context.repo,
    head: `${context.repo.owner}:${branch}`,
    state: "open",
  });
  let pull = open.data[0];
  if (pull) {
    await github.rest.pulls.update({ ...context.repo, pull_number: pull.number, title, body });
  } else {
    const base = context.payload.repository.default_branch;
    ({ data: pull } = await github.rest.pulls.create({
      ...context.repo,
      head: branch,
      base,
      title,
      body,
    }));
  }
  const issue = { ...context.repo, issue_number: pull.number };
  if (labels?.length) await github.rest.issues.addLabels({ ...issue, labels });
  if (config.assignees?.length) {
    await github.rest.issues.addAssignees({ ...issue, assignees: config.assignees });
  }
  if (note) await github.rest.issues.createComment({ ...issue, body: `${mention()}${note}` });
  return pull;
}

// What a workflow tells the assignees of a pull request it would merge but cannot.
const HELD =
  "The default branch requires checks or a merge queue, which a pull request this workflow opens cannot pass alone: approve its checks where GitHub holds them, and merge it.";

// Whether a workflow that `wanted` its pull request merged merges it: with an app's token, through
// auto-merge, which a merge queue takes too; with its own, only where the default branch's rules
// require no checks, which GitHub holds for a maintainer on the pull requests that token opens, and
// no merge queue, which refuses a merge that skips it.
async function merges({ github, context, core }, wanted, app) {
  if (!wanted || app) return wanted;
  const { data: repository } = await github.rest.repos.get(context.repo);
  const branch = repository.default_branch;
  const { data: rules } = await github.rest.repos.getBranchRules({
    ...context.repo,
    branch,
    per_page: 100,
  });
  const gates = ["required_status_checks", "merge_queue"];
  if (!rules.some((rule) => gates.includes(rule.type))) return true;
  core.warning(
    `${branch} requires checks or a merge queue, which this run's own token cannot pass: a person merges, or the automation's app does, with BUMP_APP_ID and BUMP_APP_KEY.`,
  );
  return false;
}

// The message of the commit a workflow puts on a pull request's branch, headed `headline`. One it
// merges at once, with no app to hold it for its required checks, skips them: they would start on a
// branch already merged and gone.
function message(headline, { merging, app }) {
  return merging && !app
    ? { headline, body: "Merged at once by the workflow. [skip ci]" }
    : { headline };
}

// Merges `pull`, whose gate has passed, and tells the assignees, `said` first: with an app's token,
// by GitHub's auto-merge once its required checks pass; else at once, and its branch goes. The
// squashed commit is the pull request's title and `said`, so no marker of its branch's commit
// reaches the default branch.
async function land({ github, context }, pull, { app, branch, said }) {
  const issue = { ...context.repo, issue_number: pull.number };
  if (app) {
    await github.graphql(
      `mutation($id: ID!) { enablePullRequestAutoMerge(input: { pullRequestId: $id, mergeMethod: SQUASH }) { clientMutationId } }`,
      { id: pull.node_id },
    );
    const body = `${mention()}${said} It merges once its required checks pass.`;
    await github.rest.issues.createComment({ ...issue, body });
    return;
  }
  await github.rest.pulls.merge({
    ...context.repo,
    pull_number: pull.number,
    merge_method: "squash",
    commit_title: `${pull.title} (#${pull.number})`,
    commit_message: said,
  });
  await github.rest.git.deleteRef({ ...context.repo, ref: `heads/${branch}` });
  await github.rest.issues.createComment({ ...issue, body: `${mention()}${said} Merged.` });
}

module.exports = {
  settings,
  mention,
  find,
  hold,
  release,
  propose,
  HELD,
  merges,
  message,
  land,
};
