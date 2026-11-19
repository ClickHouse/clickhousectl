import copy
import importlib.util
import io
import os
import sys
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest.mock import patch
from urllib.parse import quote

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
from release_artifacts import ProvenanceError

SPEC = importlib.util.spec_from_file_location(
    "check_release_source", SCRIPTS / "check-release-source.py"
)
source = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = source
SPEC.loader.exec_module(source)

COMMIT = "a" * 40
HEAD = "b" * 40
TREE = "c" * 40
BASE = "d" * 40
REF = "refs/tags/v1.2.3"
PREFIX = f"repos/{source.REPOSITORY}"


class Evidence:
    def __init__(self, branch="main"):
        self.branch = branch
        self.data = {
            PREFIX: {"id": source.REPOSITORY_ID, "owner": {"id": source.OWNER_ID}},
            f"{PREFIX}/git/ref/tags/v1.2.3": {
                "object": {"type": "commit", "sha": COMMIT}
            },
            f"{PREFIX}/rules/branches/{quote(branch, safe='')}": [
                {
                    "type": kind,
                    "ruleset_source_type": "Organization",
                    "ruleset_source": "ClickHouse",
                    "parameters": {
                        "required_approving_review_count": 1,
                        "dismiss_stale_reviews_on_push": True,
                        "require_last_push_approval": True,
                    },
                }
                for kind in ("deletion", "non_fast_forward", "pull_request")
            ],
            f"{PREFIX}/compare/{COMMIT}...{quote(branch, safe='')}": {
                "status": "identical",
                "behind_by": 0,
                "merge_base_commit": {"sha": COMMIT},
            },
            f"{PREFIX}/compare/main...{COMMIT}?per_page=1": {
                "status": "ahead",
                "merge_base_commit": {"sha": BASE},
            },
            f"{PREFIX}/collaborators/reviewer/permission": {"permission": "write"},
        }
        self.tag_reads = 0
        self.final_tag = COMMIT
        self.add_commit(COMMIT, HEAD, BASE, 7)

    def add_commit(self, sha, head, parent, number):
        self.data[f"{PREFIX}/commits/{sha}/pulls?per_page=100"] = [{"number": number}]
        self.data[f"{PREFIX}/pulls/{number}"] = {
            "number": number,
            "merged": True,
            "merged_at": "2026-11-18T12:00:00Z",
            "merge_commit_sha": sha,
            "base": {"ref": self.branch, "repo": {"id": source.REPOSITORY_ID}},
            "head": {"sha": head},
            "user": {"login": "author"},
            "commits": 1,
        }
        self.data[f"{PREFIX}/pulls/{number}/commits?per_page=100"] = [
            {"author": {"login": "author"}, "committer": {"login": "author"}}
        ]
        self.data[f"{PREFIX}/pulls/{number}/reviews?per_page=100"] = [
            {
                "id": number * 100,
                "state": "APPROVED",
                "commit_id": head,
                "submitted_at": "2026-11-18T11:59:00Z",
                "user": {"login": "reviewer", "type": "User"},
            }
        ]
        for value in (sha, head):
            self.data[f"{PREFIX}/git/commits/{value}"] = {
                "sha": value,
                "tree": {"sha": TREE},
                "parents": [{"sha": parent}],
            }

    def __call__(self, path, *, paginate=False):
        if path == f"{PREFIX}/git/ref/tags/v1.2.3":
            self.tag_reads += 1
            if self.tag_reads > 1:
                return {"object": {"type": "commit", "sha": self.final_tag}}
        if path not in self.data:
            raise AssertionError(f"Unexpected API path: {path}")
        return copy.deepcopy(self.data[path])

    @property
    def pull(self):
        return self.data[f"{PREFIX}/pulls/7"]

    @property
    def review(self):
        return self.data[f"{PREFIX}/pulls/7/reviews?per_page=100"][0]

    @property
    def rules(self):
        return self.data[f"{PREFIX}/rules/branches/{quote(self.branch, safe='')}"]


class AuthorizeTests(unittest.TestCase):
    def rejected(self, evidence):
        with self.assertRaises(source.SourceRejected) as error:
            source.authorize(COMMIT, REF, api=evidence)
        self.assertEqual(error.exception.code, "source_rejected")

    def test_authorizes_exact_reviewed_main_tree(self):
        result = source.authorize(COMMIT, REF, api=Evidence())
        self.assertEqual(result["source_sha"], COMMIT)
        self.assertEqual(result["source_tree"], TREE)
        self.assertEqual(result["pull_request"], 7)
        self.assertEqual(result["approval"]["head_sha"], HEAD)

    def test_accepts_annotated_release_tag(self):
        evidence = Evidence()
        tag_object = "e" * 40
        evidence.data[f"{PREFIX}/git/ref/tags/v1.2.3"]["object"] = {
            "type": "tag",
            "sha": tag_object,
        }
        evidence.data[f"{PREFIX}/git/tags/{tag_object}"] = {
            "object": {"type": "commit", "sha": COMMIT}
        }
        self.assertEqual(
            source.authorize(COMMIT, REF, api=evidence)["source_sha"], COMMIT
        )

    def test_rejects_tag_movement_during_authorization(self):
        evidence = Evidence()
        evidence.final_tag = BASE
        self.rejected(evidence)

    def test_rejects_wrong_repository_or_owner(self):
        for field in ("repository", "owner"):
            with self.subTest(field=field):
                evidence = Evidence()
                if field == "owner":
                    evidence.data[PREFIX]["owner"]["id"] = 1
                else:
                    evidence.data[PREFIX]["id"] = 1
                self.rejected(evidence)

    def test_rejects_unmerged_unrelated_and_intermediate_commits(self):
        for field in ("unmerged", "wrong_base", "intermediate", "foreign_repo"):
            with self.subTest(field=field):
                evidence = Evidence()
                if field == "unmerged":
                    evidence.pull["merged"] = False
                elif field == "wrong_base":
                    evidence.pull["base"]["ref"] = "feature/unreviewed"
                elif field == "foreign_repo":
                    evidence.pull["base"]["repo"]["id"] = 1
                else:
                    evidence.pull["merge_commit_sha"] = BASE
                self.rejected(evidence)

    def test_rejects_ambiguous_exact_merged_prs(self):
        evidence = Evidence()
        extra = copy.deepcopy(evidence.pull)
        extra["number"] = 8
        evidence.data[f"{PREFIX}/pulls/8"] = extra
        evidence.data[f"{PREFIX}/commits/{COMMIT}/pulls?per_page=100"].append(
            {"number": 8}
        )
        self.rejected(evidence)

    def test_rejects_stale_after_merge_bot_or_self_approval(self):
        for kind in ("stale", "late", "bot", "self", "committer", "read_only"):
            with self.subTest(kind=kind):
                evidence = Evidence()
                if kind == "stale":
                    evidence.review["commit_id"] = BASE
                elif kind == "late":
                    evidence.review["submitted_at"] = "2026-11-18T12:01:00Z"
                elif kind == "bot":
                    evidence.review["user"]["type"] = "Bot"
                elif kind == "self":
                    evidence.pull["user"]["login"] = "reviewer"
                elif kind == "committer":
                    evidence.data[f"{PREFIX}/pulls/7/commits?per_page=100"][0][
                        "committer"
                    ]["login"] = "reviewer"
                else:
                    evidence.data[f"{PREFIX}/collaborators/reviewer/permission"] = {
                        "permission": "read"
                    }
                self.rejected(evidence)

    def test_rejects_missing_pr_author_identity(self):
        for author in (
            None,
            {},
            {"login": None},
            {"login": ""},
            {"login": " "},
            {"login": 42},
        ):
            with self.subTest(author=author):
                evidence = Evidence()
                evidence.pull["user"] = author
                self.rejected(evidence)

    def test_rejects_missing_linked_commit_identities(self):
        for field in ("author", "committer"):
            for missing in ("absent", "null"):
                with self.subTest(field=field, missing=missing):
                    evidence = Evidence()
                    commit = evidence.data[f"{PREFIX}/pulls/7/commits?per_page=100"][0]
                    if missing == "absent":
                        del commit[field]
                    else:
                        commit[field] = None
                    self.rejected(evidence)

    def test_rejects_malformed_linked_commit_logins(self):
        for field in ("author", "committer"):
            for identity in (
                {},
                {"login": None},
                {"login": 1},
                {"login": ""},
                {"login": " "},
            ):
                with self.subTest(field=field, identity=identity):
                    evidence = Evidence()
                    commit = evidence.data[f"{PREFIX}/pulls/7/commits?per_page=100"][0]
                    commit[field] = identity
                    self.rejected(evidence)

    def test_excludes_linked_contributors_from_review(self):
        for field in ("author", "committer"):
            for identity_type in ("User", "Bot"):
                with self.subTest(field=field, identity_type=identity_type):
                    evidence = Evidence()
                    evidence.data[f"{PREFIX}/pulls/7/commits?per_page=100"][0][
                        field
                    ] = {"login": "REVIEWER", "type": identity_type}
                    self.rejected(evidence)

    def test_accepts_linked_bot_contributor_with_independent_human_reviewer(self):
        evidence = Evidence()
        evidence.data[f"{PREFIX}/pulls/7/commits?per_page=100"][0]["committer"] = {
            "login": "release-bot",
            "type": "Bot",
        }
        self.assertEqual(
            source.authorize(COMMIT, REF, api=evidence)["approval"]["reviewer"],
            "reviewer",
        )

    def test_later_change_request_or_dismissal_revokes_approval(self):
        for state in ("CHANGES_REQUESTED", "DISMISSED"):
            with self.subTest(state=state):
                evidence = Evidence()
                later = copy.deepcopy(evidence.review)
                later["state"] = state
                evidence.data[f"{PREFIX}/pulls/7/reviews?per_page=100"].append(later)
                self.rejected(evidence)

    def test_later_comment_does_not_revoke_approval(self):
        evidence = Evidence()
        comment = copy.deepcopy(evidence.review)
        comment["state"] = "COMMENTED"
        evidence.data[f"{PREFIX}/pulls/7/reviews?per_page=100"].append(comment)
        self.assertEqual(
            source.authorize(COMMIT, REF, api=evidence)["approval"]["reviewer"],
            "reviewer",
        )

    def test_rejects_merge_tree_not_approved_at_head(self):
        evidence = Evidence()
        evidence.data[f"{PREFIX}/git/commits/{COMMIT}"]["tree"]["sha"] = "e" * 40
        self.rejected(evidence)

    def test_rejects_repository_only_or_stale_org_protection(self):
        for kind in (
            "repo_only",
            "foreign_org",
            "no_last_push",
            "no_stale_dismissal",
            "missing_floor",
        ):
            with self.subTest(kind=kind):
                evidence = Evidence()
                if kind == "repo_only":
                    for rule in evidence.rules:
                        rule["ruleset_source_type"] = "Repository"
                elif kind == "foreign_org":
                    for rule in evidence.rules:
                        rule["ruleset_source"] = "OtherOrg"
                elif kind == "missing_floor":
                    evidence.rules.pop(0)
                else:
                    key = (
                        "require_last_push_approval"
                        if kind == "no_last_push"
                        else "dismiss_stale_reviews_on_push"
                    )
                    evidence.rules[-1]["parameters"][key] = False
                self.rejected(evidence)

    def test_rejects_commit_no_longer_contained_in_source_branch(self):
        evidence = Evidence()
        evidence.data[f"{PREFIX}/compare/{COMMIT}...main"]["behind_by"] = 1
        self.rejected(evidence)

    def test_authorizes_reviewed_linear_patch_history(self):
        evidence = Evidence("release/0.5.x")
        previous = "e" * 40
        previous_head = "f" * 40
        evidence.data[f"{PREFIX}/git/commits/{COMMIT}"]["parents"] = [{"sha": previous}]
        evidence.add_commit(previous, previous_head, BASE, 8)
        result = source.authorize(COMMIT, REF, api=evidence)
        self.assertEqual(result["base_branch"], "release/0.5.x")
        self.assertEqual(
            [entry["sha"] for entry in result["release_history"]["commits"]],
            [COMMIT, previous],
        )
        self.assertEqual(result["release_history"]["main_fork_sha"], BASE)

    def test_release_history_limit(self):
        evidence = Evidence("release/0.5.x")
        commits = [COMMIT] + [f"{i:040x}" for i in range(1, 201)]
        evidence.data[f"{PREFIX}/git/commits/{COMMIT}"]["parents"] = [
            {"sha": commits[1]}
        ]
        for i in range(1, 201):
            parent = commits[i + 1] if i < 200 else BASE
            evidence.add_commit(commits[i], f"{i + 1000:040x}", parent, i + 7)

        last_allowed = evidence.data[f"{PREFIX}/git/commits/{commits[199]}"]
        last_allowed["parents"] = [{"sha": BASE}]
        result = source.authorize(COMMIT, REF, api=evidence)
        self.assertEqual(len(result["release_history"]["commits"]), 200)

        last_allowed["parents"] = [{"sha": commits[200]}]
        self.rejected(evidence)

    def test_rejects_unreviewed_patch_branch_ancestry(self):
        evidence = Evidence("release/0.5.x")
        unreviewed = "e" * 40
        evidence.data[f"{PREFIX}/git/commits/{COMMIT}"]["parents"] = [
            {"sha": unreviewed}
        ]
        evidence.data[f"{PREFIX}/commits/{unreviewed}/pulls?per_page=100"] = []
        self.rejected(evidence)

    def test_rejects_non_linear_patch_history(self):
        evidence = Evidence("release/0.5.x")
        evidence.data[f"{PREFIX}/git/commits/{COMMIT}"]["parents"].append({"sha": HEAD})
        self.rejected(evidence)

    def test_propagates_transport_failure(self):
        evidence = Evidence()

        def unavailable(path, *, paginate=False):
            if path == f"{PREFIX}/pulls/7/reviews?per_page=100":
                raise ProvenanceError("Transport unavailable")
            return evidence(path, paginate=paginate)

        with self.assertRaises(ProvenanceError) as error:
            source.authorize(COMMIT, REF, api=unavailable)
        self.assertNotIsInstance(error.exception, source.SourceRejected)


class MainTests(unittest.TestCase):
    def test_dispatch_or_wrong_caller_cannot_produce_authorization(self):
        valid = {
            "GITHUB_EVENT_NAME": "push",
            "GITHUB_REPOSITORY_ID": str(source.REPOSITORY_ID),
            "GITHUB_REPOSITORY_OWNER_ID": str(source.OWNER_ID),
        }
        for change in (
            {"GITHUB_EVENT_NAME": "workflow_dispatch"},
            {"GITHUB_REPOSITORY_ID": "1"},
        ):
            with self.subTest(change=change), patch.dict(os.environ, valid | change):
                output = io.StringIO()
                with redirect_stdout(output), redirect_stderr(io.StringIO()):
                    status = source.main(["--sha", COMMIT, "--ref", REF])
                self.assertEqual(status, 1)
                self.assertEqual(output.getvalue(), "")


if __name__ == "__main__":
    unittest.main()
