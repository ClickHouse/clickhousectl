# Local command

Local commands run on this machine and never call the Cloud API.

1. **Definition.** Add the variant to its enum in `src/local/cli.rs`: `LocalCommands`, `ServerCommands` or
   `PostgresCommands`.
2. **Dispatch.** Add the match arm. `LocalCommands` dispatches in `run()` in `src/local/mod.rs`,
   `ServerCommands` in `run_server_commands()` in the same file, and `PostgresCommands` in `run()` in
   `src/local/postgres.rs`.
3. **Handler.** Put the logic in the module that owns the area (`server.rs`, `postgres.rs`, `docker.rs`, ...) or
   a new module under `src/local/`. Keep it out of `main.rs`.
4. **Output.** Every command supports `--json`. Add an output struct to `src/local/output.rs` that derives
   `Serialize` and implements `Display` for the human view. Print it with `output::print_output(&out, json)`.
5. **Errors.** Return `crate::error::Error`. Map each new variant in `LocalErrorOutput::from_error` in
   `src/local/output.rs`. That match has no catch-all, so the build fails until you do. Use `Mapping::parity`
   (show the message as is) by default. Use `Mapping::redacted` when the message includes text from another
   program, such as subprocess stderr, Docker or OS errors, or download bodies. That text can contain paths,
   SQL or credentials.
6. **Tests.** Test behaviour that needs the real binary in a new `tests/local_<area>_<concern>_test.rs`. This
   covers files, processes, exit codes, and JSON on stdout or stderr. Don't add local tests to
   `tests/cli_request_shape_test.rs`, which is for Cloud only. Test pure helpers in an inline `mod tests`.
