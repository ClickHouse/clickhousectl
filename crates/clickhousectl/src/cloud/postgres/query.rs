use crate::cloud::client::{CloudClient, CloudError, Result as CloudResult};
use crate::cloud::shared::{NameSelector, NamedResource, resolve_org_id};
use crate::failure::{self, ApiFailure, FailureKind, FailureStage};
use clickhouse_cloud_api::RunPostgresQueryRequest;
use futures_util::StreamExt;
use std::io::{IsTerminal, Read, Write};
use std::path::Path;

fn build_postgres_query_request(sql: String, database: Option<&str>) -> RunPostgresQueryRequest {
    RunPostgresQueryRequest {
        sql,
        database: database.map(str::to_string),
    }
}

fn decode_postgres_query_sql(bytes: Vec<u8>, source: &str) -> CloudResult<String> {
    let sql = String::from_utf8(bytes).map_err(|_| {
        CloudError::new(format!("SQL from {source} must be UTF-8 text"))
            .with_failure(ApiFailure::new(FailureKind::Io))
    })?;
    if sql.trim().is_empty() {
        return Err(CloudError::usage(format!("No SQL provided by {source}")));
    }
    Ok(sql)
}

fn read_postgres_query_sql(
    inline: Option<&str>,
    queries_file: Option<&Path>,
    mut stdin: impl Read,
    stdin_is_terminal: bool,
) -> CloudResult<String> {
    if let Some(sql) = inline {
        return decode_postgres_query_sql(sql.as_bytes().to_vec(), "--query");
    }
    if let Some(path) = queries_file.filter(|path| *path != Path::new("-")) {
        let bytes = std::fs::read(path).map_err(|error| {
            CloudError::new(format!(
                "Failed to read SQL file '{}': {error}",
                path.display()
            ))
            .with_failure(ApiFailure::new(FailureKind::Io))
        })?;
        return decode_postgres_query_sql(bytes, &format!("SQL file '{}'", path.display()));
    }
    if queries_file.is_none() && stdin_is_terminal {
        return Err(CloudError::usage(
            "No SQL provided. Pass --query, --queries-file, or pipe SQL on stdin.",
        ));
    }
    let mut bytes = Vec::new();
    stdin.read_to_end(&mut bytes)?;
    decode_postgres_query_sql(bytes, "stdin")
}

pub(super) async fn postgres_query(
    client: &CloudClient,
    selector: &NameSelector,
    inline: Option<&str>,
    queries_file: Option<&Path>,
    database: Option<&str>,
    json: bool,
) -> CloudResult<()> {
    failure::start_span();
    if !client.is_bearer_auth() {
        return Err(CloudError::auth(
            "Postgres queries require OAuth authentication; API keys are unsupported. \
             Run `clickhousectl cloud auth login`, then remove higher-priority API key \
             flags, project credentials, and CLICKHOUSE_CLOUD_API_KEY / \
             CLICKHOUSE_CLOUD_API_SECRET environment variables so OAuth is selected.",
        )
        .at_stage(FailureStage::QueryRequest));
    }
    let stdin = std::io::stdin();
    let sql = read_postgres_query_sql(inline, queries_file, stdin.lock(), stdin.is_terminal())
        .map_err(|error| error.at_stage(FailureStage::SqlInput))?;
    let org_id = resolve_org_id(client)
        .await
        .map_err(|error| error.at_stage(FailureStage::OrgResolution))?;
    let service_id = selector
        .resolve(client, NamedResource::Postgres)
        .await
        .map_err(|error| error.at_stage(FailureStage::ServiceResolution))?;
    let request = build_postgres_query_request(sql, database);
    let response = client
        .run_postgres_query(&org_id, &service_id, &request)
        .await
        .map_err(|error| error.at_stage(FailureStage::QueryRequest))?;

    let mut stream = response.bytes_stream();
    let mut output = crate::stdout::stdout();
    let mut renderer = PostgresQueryRenderer::default();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| {
            CloudError::new(format!("Failed to read Postgres query response: {error}"))
                .with_failure(ApiFailure::new(FailureKind::Transport))
                .at_stage(FailureStage::ResponseStream)
        })?;
        if json {
            // Preserve the server's array lines byte-for-byte, including
            // duplicate columns and numeric values outside Rust's range.
            output
                .write_all(&chunk)
                .map_err(|error| CloudError::from(error).at_stage(FailureStage::ResponseStream))?;
        } else {
            renderer
                .push(&chunk, &mut output)
                .map_err(|error| error.at_stage(FailureStage::ResponseStream))?;
        }
        output
            .flush()
            .map_err(|error| CloudError::from(error).at_stage(FailureStage::ResponseStream))?;
    }
    if !json {
        renderer
            .finish(&mut output)
            .map_err(|error| error.at_stage(FailureStage::ResponseStream))?;
    }
    output
        .flush()
        .map_err(|error| CloudError::from(error).at_stage(FailureStage::ResponseStream))?;
    Ok(())
}

/// Render JSONCompactEachRowWithNamesAndTypes a line at a time. The first two
/// arrays describe columns; subsequent arrays retain position and duplicate
/// column names. Only an unfinished line is buffered across response chunks.
#[derive(Default)]
struct PostgresQueryRenderer {
    pending: Vec<u8>,
    columns: Option<usize>,
    types_seen: bool,
}

impl PostgresQueryRenderer {
    fn push(&mut self, bytes: &[u8], output: &mut impl Write) -> CloudResult<()> {
        for part in bytes.split_inclusive(|byte| *byte == b'\n') {
            self.pending.extend_from_slice(part);
            if part.last() == Some(&b'\n') {
                self.render_line(output)?;
                self.pending.clear();
            }
        }
        Ok(())
    }

    fn finish(&mut self, output: &mut impl Write) -> CloudResult<()> {
        if !self.pending.is_empty() {
            self.render_line(output)?;
            self.pending.clear();
        }
        if self.columns.is_some() && !self.types_seen {
            return Err(CloudError::new(
                "Postgres query response is missing column types",
            ));
        }
        Ok(())
    }

    fn render_line(&mut self, output: &mut impl Write) -> CloudResult<()> {
        if self.pending.iter().all(u8::is_ascii_whitespace) {
            return Ok(());
        }
        let cells: Vec<serde_json::Value> =
            serde_json::from_slice(&self.pending).map_err(|error| {
                CloudError::new(format!("Invalid Postgres query response: {error}"))
            })?;
        if let Some(columns) = self.columns {
            if cells.len() != columns {
                return Err(CloudError::new(format!(
                    "Postgres query response has {} values for {columns} columns",
                    cells.len()
                )));
            }
            if !self.types_seen {
                if !cells.iter().all(serde_json::Value::is_string) {
                    return Err(CloudError::new(
                        "Postgres query column types must be strings",
                    ));
                }
                self.types_seen = true;
                return Ok(());
            }
        } else {
            if !cells.iter().all(serde_json::Value::is_string) {
                return Err(CloudError::new(
                    "Postgres query column names must be strings",
                ));
            }
            self.columns = Some(cells.len());
        }
        for (index, cell) in cells.iter().enumerate() {
            if index > 0 {
                output.write_all(b"\t")?;
            }
            match cell {
                serde_json::Value::String(value)
                    if !value.is_empty()
                        && !value
                            .chars()
                            .any(|c| c.is_control() || matches!(c, '"' | '\\')) =>
                {
                    output.write_all(value.as_bytes())?;
                }
                serde_json::Value::Null => output.write_all(b"NULL")?,
                _ => output.write_all(serde_json::to_string(cell)?.as_bytes())?,
            }
        }
        output.write_all(b"\n")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_postgres_query_request_minimal_and_maximal() {
        let minimal = build_postgres_query_request("SELECT 1".into(), None);
        assert_eq!(minimal.sql, "SELECT 1");
        assert!(minimal.database.is_none());
        let maximal =
            build_postgres_query_request("SELECT '雪';\nSELECT 2".into(), Some("reporting"));
        assert_eq!(maximal.sql, "SELECT '雪';\nSELECT 2");
        assert_eq!(maximal.database.as_deref(), Some("reporting"));
    }

    #[test]
    fn postgres_query_inline_never_reads_stdin() {
        struct UnreadableStdin;
        impl Read for UnreadableStdin {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                panic!("inline SQL must not read stdin");
            }
        }
        assert_eq!(
            read_postgres_query_sql(Some("SELECT 1"), None, UnreadableStdin, false).unwrap(),
            "SELECT 1"
        );
        assert!(read_postgres_query_sql(Some(" \n"), None, UnreadableStdin, false).is_err());
    }

    #[test]
    fn postgres_query_reads_file_and_both_stdin_forms() {
        for source in [None, Some(Path::new("-"))] {
            assert_eq!(
                read_postgres_query_sql(None, source, "SELECT '雪'".as_bytes(), false).unwrap(),
                "SELECT '雪'"
            );
        }
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"SELECT 2\n").unwrap();
        assert_eq!(
            read_postgres_query_sql(None, Some(file.path()), std::io::empty(), false).unwrap(),
            "SELECT 2\n"
        );
        for sql in [vec![], b" \n\t".to_vec(), vec![0xff]] {
            assert!(read_postgres_query_sql(None, None, sql.as_slice(), false).is_err());
        }
        assert!(read_postgres_query_sql(None, None, std::io::empty(), true).is_err());
        let error = read_postgres_query_sql(
            None,
            Some(Path::new("missing-query-file.sql")),
            std::io::empty(),
            false,
        )
        .unwrap_err();
        assert_eq!(error.failure.unwrap().kind, FailureKind::Io);
    }

    #[test]
    fn postgres_query_human_renderer_handles_arbitrary_chunks_and_duplicate_names() {
        let body = "[\"value\",\"value\",\"text\"]\n[\"Int64\",\"Nullable(Int64)\",\"String\"]\n[1,null,\"雪\"]\n[2,3,\"line\\nwith\\ttabs\"]";
        for chunk_size in [1, 2, 7, body.len()] {
            let mut renderer = PostgresQueryRenderer::default();
            let mut output = Vec::new();
            for chunk in body.as_bytes().chunks(chunk_size) {
                renderer.push(chunk, &mut output).unwrap();
            }
            renderer.finish(&mut output).unwrap();
            assert_eq!(
                String::from_utf8(output).unwrap(),
                "value\tvalue\ttext\n1\tNULL\t雪\n2\t3\t\"line\\nwith\\ttabs\"\n"
            );
        }
    }

    #[test]
    fn postgres_query_human_renderer_accepts_empty_results() {
        for body in ["", "[\"id\"]\n[\"Int64\"]\n"] {
            let mut renderer = PostgresQueryRenderer::default();
            let mut output = Vec::new();
            renderer.push(body.as_bytes(), &mut output).unwrap();
            renderer.finish(&mut output).unwrap();
            assert_eq!(
                output,
                if body.is_empty() {
                    b"".as_slice()
                } else {
                    b"id\n".as_slice()
                }
            );
        }
    }

    #[test]
    fn postgres_query_human_renderer_rejects_malformed_shapes() {
        for body in [
            "{}\n",
            "[1]\n",
            "[\"id\"]\n",
            "[\"id\"]\n[1]\n",
            "[\"id\"]\n[]\n",
            "[\"id\"]\n[\"Int64\"]\n[1,2]\n",
            "[\"id\"]\n[\"Int64\"]\n{\"id\":1}\n",
            "[\"id\"]\n[\"Int64\"]\n[",
        ] {
            let mut renderer = PostgresQueryRenderer::default();
            let mut output = Vec::new();
            let result = renderer
                .push(body.as_bytes(), &mut output)
                .and_then(|_| renderer.finish(&mut output));
            assert!(result.is_err(), "should reject {body:?}");
        }
    }
}
