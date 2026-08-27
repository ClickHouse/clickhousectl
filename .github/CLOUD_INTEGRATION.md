# Cloud integration decisions

`Cloud Integration` remains on demand for pull requests. The required
`Cloud integration decision` check is bound to one exact PR head SHA and passes
only after one of these outcomes:

1. Apply the `run-cloud-integration` label and all suites selected by the
   trusted planner pass.
2. Apply the label and the trusted planner selects no live suites. The
   environment-bearing job stays skipped.
3. A maintainer records a one-shot override for the current full head SHA.

After a push, remove and reapply `run-cloud-integration` to start a run for the
new SHA. Earlier live results and overrides do not carry forward. Fork PRs do
not receive Cloud secrets; use a same-repository mirror or the explicit
maintainer override.

## Maintainer override

Repository administrators and maintainers can comment on an open PR:

```text
/cloud-integration-override <full-current-head-sha> <reason-or-stack-run-url>
```

The controller verifies effective `maintain` or `admin` permission and that the
40-character SHA is still the PR head. The command comment, controller reply,
and check output record the actor, SHA, event timestamp, and reason. A new SHA
requires a new command. Writers, unrelated comments, stale SHAs, and persistent
labels cannot create an override.

Use an override after a failed live run only as an explicit subsequent
maintainer decision. Include why the failure is accepted or link the successful
run that covers the change.

An override is not sticky for the same SHA. A later admitted Cloud Integration
run replaces it as the current decision. If that run fails, the decision stays
failing until the run succeeds or a maintainer records a subsequent override.

A PR that changes `.github/workflows/cloud-integration.yml` cannot use that
modified workflow to attest itself. Its decision remains failing until a
maintainer reviews the workflow change and records an exact-SHA override.

A PR that changes `scripts/classify-cloud-integration.py` can select different
suites in the PR run than the trusted default-branch controller derives. Review
the classifier change and use an exact-SHA override when that expected mismatch
prevents the run from satisfying the decision.

## Stacked pull requests

Run affected suites independently on every PR by default. A manual `all` run on
the top branch validates that combined snapshot only; it does not automatically
cover lower PR head SHAs.

When one successful top-stack run intentionally covers lower PRs, a maintainer
must add a separate exact-SHA override to every lower PR and link that run in
the reason. Repeat the run or override for any lower PR whose head changes.
This records the coverage decision on every mergeable SHA instead of silently
transferring a status through the stack.

The manual top-stack run is evidence, not an automatic PR decision. Satisfy the
top PR itself with its label-triggered run or an exact-SHA override that links
the successful manual run.

## Post-merge rollout

Do not require the check before this workflow is on the default branch. After
merging it to `main`:

1. Open or synchronize a representative PR and confirm that GitHub Actions
   creates `Cloud integration decision` on its current head SHA.
2. Exercise either the `run-cloud-integration` label or an exact-SHA override
   and confirm that the same check becomes successful.
3. In the `main` branch protection rule or ruleset, add
   `Cloud integration decision` from the GitHub Actions app to the required
   status checks. Do not make the `Cloud Integration` workflow itself required.

Existing open PRs with no decision check must be synchronized/reopened, run via
the label, or given an exact-SHA override before enabling the rule. This order
prevents branch protection from locking every PR while the controller is not
yet available.
