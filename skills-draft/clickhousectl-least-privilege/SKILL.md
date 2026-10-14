---
name: clickhousectl-least-privilege
description: Gets a scoped ClickHouse Cloud API key with the fewest permissions a task needs, then runs the task with `clickhousectl`. Starting from the user's OAuth login, it picks the organization, reads which permissions each planned command needs, chooses the roles that cover them while granting the least extra access, and asks a human to approve them with `cloud access request`. Use when a Cloud task needs write access and no API key is configured, or when a command fails with permission denied under a key from this flow.
---

# Negotiate least-privilege access for a clickhousectl task

You never grant access yourself. You work out which roles to propose, and a human approves them.

## 1. Check how the CLI is authenticated

```bash
clickhousectl cloud auth status --json
```

In `sources`, find the row with `"active": "yes"`:

- **`API key`, `Env vars` or `CLI flags`**: an API key is already active. Skip this process and run the task
  with it.
- **`OAuth`**: continue to step 2.
- **No active row, or OAuth `Expired`**: run `clickhousectl cloud auth login`. It opens a browser and a human
  must complete it. Then check the status again.

## 2. Choose the organization

Everything below is scoped to one organization.

```bash
clickhousectl cloud org list --json
```

- One organization: use it for everything.
- More than one: ask the user which one to work in. Do not guess.

Pass `--org-id <org-id>` on every later command.

## 3. Build the role catalog

Permissions are granted through roles, and roles belong to an organization. Each role has policies. A policy
allows or denies a list of permissions, and can be limited to specific resources.

```bash
clickhousectl cloud access roles --org-id <org-id> --json
```

This returns the roles you can propose, each with all of its policies:

```json
[{"id": "<uuid>", "name": "…",
  "policies": [{"allowDeny": "ALLOW",
                "permissions": ["control-plane:service:view"],
                "resources": ["instance/<service-id>"]}]}]
```

- `resources: ["*"]` means every resource. `organization/<id>` and `instance/<service-id>` mean that one
  resource only.
- An empty list (`[]`) means there is nothing you can propose. Tell the user and stop.

## 4. Work out the permissions the task needs

When the user asks for a task, such as "start a service", list every command you will run, with its flags.
Each command's `--help` ends with a `CONTEXT FOR AGENTS:` block that declares the permissions it needs:

```bash
clickhousectl cloud service start --help
```

- `API key permission: <p>`: always needed.
- `API key (<condition>): <p>`: needed only under that condition, such as `With --force` or
  `Without --org-id`. Include it only if your plan meets the condition.
- `Authorization: …`: no role can grant this. Tell the user about it.

Note the resource each permission applies to. A service permission applies to `instance/<service-id>`, and an
organization permission to `organization/<org-id>`.

## 5. Choose the roles

Find the set of roles that:

1. covers every needed permission on its resource with an `ALLOW` policy that none of the chosen roles
   `DENY`s, and
2. grants the fewest permissions the task does not need (least privilege). If two sets are equal on that,
   choose the one with fewer roles.

Show the user the needed permissions, the roles you chose, and any extra permissions those roles grant. If no
set of roles covers everything, say which permissions are missing and stop.

## 6. Request the key

```bash
clickhousectl cloud access request --org-id <org-id> \
  --perms role:<uuid>,role:<uuid> \
  --purpose "<the task>" --no-browser --json
```

- The approval URL goes to stderr. Give it to the user; a human must approve it within 15 minutes.
- The JSON output's `grantedRoleIds` lists what the human approved, which may differ from what you
  proposed. Check that the approved roles still cover the task before you continue.
- If the request is denied or expires, tell the user and stop.

The issued key is saved and becomes the active credential. The secret is never printed.

## 7. Run the task

Run the planned commands with the new key.

## Caveats

- **Once a key is saved, it is the active credential.** To change the permission set, start a new flow.
- **An API key can't read the role catalog.** If a command fails with permission denied, ask the user
  whether to start a new authorization. If they agree, remove the saved key and start again from step 1:

  ```bash
  clickhousectl cloud auth logout --api-keys
  ```
