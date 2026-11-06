# Cloud command

`clickhouse-cloud-api` must already have the endpoint and models. If it doesn't, change the library first in
its own PR (see `crates/clickhouse-cloud-api/AGENTS.md`).

Everything for a command lives in its domain module, `src/cloud/<domain>.rs`: the clap definition, permissions,
dispatch, the `CloudClient` wrapper, the request builder, the handler and the tests. A large domain is a
directory, `src/cloud/<domain>/`: `PERMISSIONS`, `run()` and re-exports in `mod.rs`, clap definitions in
`cli.rs`, `CloudClient` wrappers in `client.rs`, and one file per command family, with tests beside the code
they test. Copy the pattern from `src/cloud/services/` or `src/cloud/postgres/`. Handlers never call
`client.api()`. The one wrapper that returns the raw library error is `run_service_query{,_bearer}` in
`services/client.rs`: query paths stream the response and classify the error themselves. Don't copy that for
other endpoints.

1. **Definition.** Add the variant and its argument struct to the domain's command enum. For a new command
   group:
   - Create `src/cloud/<domain>.rs` (or `src/cloud/<domain>/mod.rs`) and re-export its enum privately from
     `src/cloud/cli.rs`.
   - Add an arm to `CloudCommands::is_write_command()` in `src/cloud/cli.rs`.
   - Add an arm to `dispatch()` in `src/cloud/mod.rs`.
   - Add the module's `PERMISSIONS` to the list in `src/cloud/permissions.rs`.
2. **Read or write.** Classify the variant in the enum's `is_write()` match. Write commands need API key auth.
   OAuth login is read-only, so a write command run with OAuth fails before sending any request. Add a
   read/write test next to the definition.
3. **Dispatch.** Add the arm to the domain's `run()`.
4. **Permissions.** In the domain's `PERMISSIONS` table, declare every API call the command can make. Include
   reads before a write, polling, cleanup and lookups by name. Put calls that only happen sometimes in
   `Conditional` groups. `Conditional::flag` also checks that the flag exists.

   ```rust
   Permission::api("service delete", &[&op::INSTANCE_DELETE]).when(&[
       Conditional::flag("force", &[&op::INSTANCE_GET, &op::INSTANCE_STATE_UPDATE]),
       Conditional::new("Owned query-key cleanup", &[&op::OPENAPI_KEY_DELETE]),
   ])
   ```

   - `Permission::api` already covers looking up the organization. Add `.unscoped()` only if the handler never
     does that lookup.
   - Use `Permission::non_api(path, purpose)` for a command that calls no API. Add `.authorization(...)` when a
     command also needs SQL or another kind of access.
   - A parent command that also runs on its own, such as `org prometheus`, needs its own declaration.
   - Tests reject missing, duplicate and stale declarations. They don't compare the table with what the handler
     actually calls, so check that by reading the handler.
5. **Wrapper.** Add a method to the domain's `impl CloudClient` block. It calls `self.api().<method>()`,
   converts errors with `self.convert_error(e)` and unwraps the response with `Self::unwrap_response`. For
   errors that need the org ID or lookup context, use `convert_error_for_organization(e, org_id)` or
   `convert_error_for_lookup(e, lookup)` instead. Use the library's types. These conversions also record the
   failure kind for telemetry, so don't build a `CloudError` by hand.
   - `convert_error_for_lookup` reports any 400 or 404 over well-formed IDs as a missing resource. Use it only
     when the path names the resource itself and the request has no other input, such as a get or delete.
   - For a subresource, or a request with a body, a 404 can mean the subresource is absent and a 400 can be a
     validation error. Confirm the parent with its own GET first, as `convert_postgres_by_id_result` in
     `postgres/client.rs` does, and declare that GET as a `Conditional` permission.
6. **Request builder.** If the command sends a body, write `build_<name>_request(...)` returning the library's
   request struct. Unit-test it with minimal and maximal input, asserting on the struct's fields.
7. **Handler.** Call the builder, then the wrapper, then print:

   ```rust
   if json { println!("{}", serde_json::to_string_pretty(&data)?); } else { print_human(&data)?; }
   ```

   - Show a single resource with `print_human`. It hides deprecated fields and summarises certificates and keys
     instead of printing them in full. Show lists with `tabled`, and short confirmations with `println!`.
   - Every response field is an `Option`. Never `unwrap()` one. Print a missing value with `output::or_absent`,
     which prints `-`. A `--filter` treats a missing value as not matching.
8. **Tests.** Test runtime behaviour that the parse and builder tests don't reach in
   `tests/cli_request_shape_test.rs`. This covers auth, multi-call flows, errors and output. Those tests run
   the real binary against a wiremock server.
