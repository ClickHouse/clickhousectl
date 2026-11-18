#!/usr/bin/env python3
"""Authorize a release commit before a trusted builder checks out source.

This makes read-only GitHub API calls. It neither builds code nor grants AWS
credentials. Invoke the copy from the immutable builder revision, never the
copy checked out from the release tag being evaluated.
"""

import argparse
from datetime import datetime
import json
import os
import re
import sys
from urllib.parse import quote

from release_artifacts import ProvenanceError, github_api

REPOSITORY = "ClickHouse/clickhousectl"
REPOSITORY_ID = 1150809931
OWNER_ID = 54801242
SHA_RE = re.compile(r"[0-9a-f]{40}\Z")
TAG_RE = re.compile(r"refs/tags/v[0-9]+\.[0-9]+\.[0-9]+\Z")
RELEASE_BRANCH_RE = re.compile(r"release/[^/]+\Z")


class SourceRejected(ProvenanceError):
    """The available evidence does not authorize this source commit."""


def require(condition, reason):
    if not condition:
        raise SourceRejected(reason)


def resolve_tag(ref, api):
    tag = ref.removeprefix("refs/tags/")
    response = api(f"repos/{REPOSITORY}/git/ref/tags/{quote(tag, safe='')}")
    require(isinstance(response, dict), "Invalid tag response")
    target = response.get("object")
    for _ in range(8):
        require(isinstance(target, dict), "Invalid tag target")
        sha = target.get("sha")
        require(isinstance(sha, str) and SHA_RE.fullmatch(sha), "Invalid tag SHA")
        if target.get("type") == "commit":
            return sha
        require(target.get("type") == "tag", "Tag does not resolve to a commit")
        response = api(f"repos/{REPOSITORY}/git/tags/{sha}")
        require(isinstance(response, dict), "Invalid annotated tag response")
        target = response.get("object")
    raise SourceRejected("Annotated tag chain exceeds the supported depth")


def check_branch(branch, sha, api):
    rules = api(f"repos/{REPOSITORY}/rules/branches/{quote(branch, safe='')}")
    require(isinstance(rules, list), "Invalid effective branch rules")
    organization_rules = [
        rule
        for rule in rules
        if isinstance(rule, dict)
        and rule.get("ruleset_source_type") == "Organization"
        and rule.get("ruleset_source") == "ClickHouse"
    ]
    types = {rule.get("type") for rule in organization_rules}
    require(
        {"deletion", "non_fast_forward", "pull_request"} <= types,
        f"{branch} lacks active organization-level branch protection",
    )
    require(
        any(
            rule.get("type") == "pull_request"
            and isinstance(rule.get("parameters"), dict)
            and type(rule["parameters"].get("required_approving_review_count")) is int
            and rule["parameters"]["required_approving_review_count"] >= 1
            and rule["parameters"].get("dismiss_stale_reviews_on_push") is True
            and rule["parameters"].get("require_last_push_approval") is True
            for rule in organization_rules
        ),
        f"{branch} does not require fresh, last-push-separated approving reviews",
    )
    comparison = api(f"repos/{REPOSITORY}/compare/{sha}...{quote(branch, safe='')}")
    require(
        isinstance(comparison, dict)
        and comparison.get("status") in {"ahead", "identical"}
        and comparison.get("behind_by") == 0
        and comparison.get("merge_base_commit", {}).get("sha") == sha,
        f"Release commit is not contained in {branch}",
    )


def timestamp(value):
    require(isinstance(value, str), "Missing review/merge timestamp")
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as error:
        raise SourceRejected("Invalid review/merge timestamp") from error
    require(parsed.tzinfo is not None, "Review/merge timestamp has no timezone")
    return parsed


def git_commit(sha, api, cache):
    if sha not in cache:
        commit = api(f"repos/{REPOSITORY}/git/commits/{sha}")
        require(
            isinstance(commit, dict)
            and commit.get("sha") == sha
            and isinstance(commit.get("tree"), dict)
            and isinstance(commit["tree"].get("sha"), str)
            and SHA_RE.fullmatch(commit["tree"]["sha"])
            and isinstance(commit.get("parents"), list),
            "Invalid Git commit evidence",
        )
        cache[sha] = commit
    return cache[sha]


def exact_head_approval(pull, api):
    number = pull["number"]
    head = pull.get("head", {}).get("sha")
    require(isinstance(head, str) and SHA_RE.fullmatch(head), "Invalid PR head")
    merged_at = timestamp(pull.get("merged_at"))
    count = pull.get("commits")
    require(type(count) is int and 0 < count <= 250, "Unsupported PR commit count")
    author = pull.get("user")
    require(
        isinstance(author, dict)
        and isinstance(author.get("login"), str)
        and bool(author["login"].strip()),
        "Missing linked PR author identity",
    )
    contributors = {author["login"].casefold()}
    commits = api(
        f"repos/{REPOSITORY}/pulls/{number}/commits?per_page=100", paginate=True
    )
    require(
        isinstance(commits, list) and len(commits) == count, "Incomplete PR commits"
    )
    for commit in commits:
        require(isinstance(commit, dict), "Invalid PR commit")
        for field in ("author", "committer"):
            user = commit.get(field)
            require(
                isinstance(user, dict)
                and isinstance(user.get("login"), str)
                and bool(user["login"].strip()),
                "Missing linked PR commit identity",
            )
            contributors.add(user["login"].casefold())
    reviews = api(
        f"repos/{REPOSITORY}/pulls/{number}/reviews?per_page=100", paginate=True
    )
    require(isinstance(reviews, list), "Invalid pull-request reviews")
    votes = {}
    for review in reviews:
        require(isinstance(review, dict), "Invalid pull-request review")
        user = review.get("user")
        if not isinstance(user, dict) or not isinstance(user.get("login"), str):
            continue
        # Comments do not replace an earlier approval or change request.
        if review.get("state") in {"APPROVED", "CHANGES_REQUESTED", "DISMISSED"}:
            votes[user["login"].casefold()] = review
    for review in votes.values():
        login = review["user"]["login"]
        if (
            review.get("state") != "APPROVED"
            or review.get("commit_id") != head
            or review["user"].get("type") != "User"
            or login.casefold() in contributors
            or timestamp(review.get("submitted_at")) > merged_at
        ):
            continue
        permission = api(
            f"repos/{REPOSITORY}/collaborators/{quote(login, safe='')}/permission"
        )
        require(isinstance(permission, dict), "Invalid reviewer permission response")
        if permission.get("permission") in {"write", "admin"}:
            return {
                "head_sha": head,
                "review_id": review.get("id"),
                "reviewer": login,
            }
    raise SourceRejected(
        f"PR #{number} lacks a pre-merge, exact-head approval from an independent human writer"
    )


def reviewed_commit(sha, api, cache, *, branch=None):
    associated = api(
        f"repos/{REPOSITORY}/commits/{sha}/pulls?per_page=100", paginate=True
    )
    require(isinstance(associated, list), "Invalid associated pull requests")
    candidates = {}
    for entry in associated:
        require(isinstance(entry, dict), "Invalid associated pull request")
        number = entry.get("number")
        require(type(number) is int and number > 0, "Invalid pull-request number")
        pull = api(f"repos/{REPOSITORY}/pulls/{number}")
        require(isinstance(pull, dict), "Invalid pull-request response")
        base = pull.get("base", {})
        base_branch = base.get("ref")
        if (
            pull.get("merged") is True
            and pull.get("merge_commit_sha") == sha
            and base.get("repo", {}).get("id") == REPOSITORY_ID
            and isinstance(base_branch, str)
            and (base_branch == "main" or RELEASE_BRANCH_RE.fullmatch(base_branch))
            and (branch is None or base_branch == branch)
        ):
            candidates[number] = pull
    require(
        len(candidates) == 1,
        "Commit needs one unambiguous exact merged PR into main or its release branch",
    )
    pull = next(iter(candidates.values()))
    approval = exact_head_approval(pull, api)
    commit = git_commit(sha, api, cache)
    head = git_commit(approval["head_sha"], api, cache)
    # Deliberately stricter than reconstructing a merge. A release must have
    # exactly the tree that was approved, including the version-bump files.
    # Refresh/review a release PR before merging if its base has moved.
    require(
        commit["tree"]["sha"] == head["tree"]["sha"],
        "Released tree differs from the approved PR head",
    )
    return {
        "sha": sha,
        "tree": commit["tree"]["sha"],
        "base_branch": pull["base"]["ref"],
        "pull_request": pull["number"],
        "approval": approval,
    }


def release_history(sha, branch, api, cache, first):
    comparison = api(f"repos/{REPOSITORY}/compare/main...{sha}?per_page=1")
    require(isinstance(comparison, dict), "Invalid main/release comparison")
    fork = comparison.get("merge_base_commit", {}).get("sha")
    require(isinstance(fork, str) and SHA_RE.fullmatch(fork), "No trusted main fork")
    cursor = sha
    evidence = []
    for _ in range(200):
        if cursor == fork:
            return {"main_fork_sha": fork, "commits": evidence}
        evidence.append(
            first
            if cursor == sha
            else reviewed_commit(cursor, api, cache, branch=branch)
        )
        parents = git_commit(cursor, api, cache)["parents"]
        require(
            len(parents) == 1,
            "Release history must be linear reviewed PR commits rooted on main",
        )
        parent = parents[0]
        require(
            isinstance(parent, dict)
            and isinstance(parent.get("sha"), str)
            and SHA_RE.fullmatch(parent["sha"]),
            "Invalid release parent",
        )
        cursor = parent["sha"]
    require(cursor == fork, "Release review history exceeds 200 commits")
    return {"main_fork_sha": fork, "commits": evidence}


def authorize(sha, ref, *, api=github_api):
    require(
        isinstance(sha, str) and SHA_RE.fullmatch(sha),
        "Source must be a full commit SHA",
    )
    require(
        isinstance(ref, str) and TAG_RE.fullmatch(ref),
        "Release ref must be refs/tags/vX.Y.Z",
    )
    repository = api(f"repos/{REPOSITORY}")
    require(
        isinstance(repository, dict)
        and repository.get("id") == REPOSITORY_ID
        and repository.get("owner", {}).get("id") == OWNER_ID,
        "Repository or organization identity changed",
    )
    require(
        resolve_tag(ref, api) == sha,
        "Release tag no longer resolves to the expected commit",
    )
    cache = {}
    reviewed = reviewed_commit(sha, api, cache)
    branch = reviewed["base_branch"]
    check_branch(branch, sha, api)
    history = None
    if branch != "main":
        history = release_history(sha, branch, api, cache, reviewed)
    # Do not reuse a cached response here: a tag that moved while evidence
    # was being collected must not be authorized.
    require(resolve_tag(ref, api) == sha, "Release tag moved during authorization")
    return {
        "schema_version": 1,
        "repository": REPOSITORY,
        "repository_id": REPOSITORY_ID,
        "owner_id": OWNER_ID,
        "source_sha": sha,
        "ref": ref,
        "base_branch": branch,
        "source_tree": reviewed["tree"],
        "pull_request": reviewed["pull_request"],
        "approval": reviewed["approval"],
        "release_history": history,
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--sha", required=True)
    parser.add_argument("--ref", required=True)
    args = parser.parse_args(argv)
    try:
        require(
            os.environ.get("GITHUB_EVENT_NAME") == "push",
            "Only push releases are authorized",
        )
        require(
            os.environ.get("GITHUB_REPOSITORY_ID") == str(REPOSITORY_ID)
            and os.environ.get("GITHUB_REPOSITORY_OWNER_ID") == str(OWNER_ID),
            "Caller repository identity does not match clickhousectl",
        )
        evidence = authorize(args.sha, args.ref)
    except (ProvenanceError, KeyError, TypeError, AttributeError) as error:
        print(f"Release source rejected: {error}", file=sys.stderr)
        return 1
    print(json.dumps(evidence, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
