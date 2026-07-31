//! Reading and splitting SQL passed to `cloud service query`.
//!
//! The Query API accepts one statement per request, while ClickHouse query
//! files may contain several semicolon-delimited statements. This module does
//! the minimum lexical work needed to find real statement delimiters without
//! trying to parse or restrict ClickHouse SQL itself.

use std::io::{IsTerminal as _, Read as _};

/// Read the command's SQL input and return the Query API requests to run.
///
/// `--query` deliberately remains a single request. Only `--queries-file`
/// (including `--queries-file -`) has query-file semantics and is split into
/// statements. Bare piped stdin also retains its existing single-request
/// behavior.
pub fn read_query_statements(
    inline: Option<&str>,
    queries_file: Option<&str>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    if let Some(query) = inline {
        if query.trim().is_empty() {
            return Err("--query was empty".into());
        }
        return Ok(vec![query.to_string()]);
    }

    if let Some(path) = queries_file {
        let mut content = String::new();
        if path == "-" {
            std::io::stdin().read_to_string(&mut content)?;
        } else {
            content = std::fs::read_to_string(path)?;
        }
        if content.trim().is_empty() {
            return Err("queries file was empty".into());
        }

        let statements = split_query_file(&content);
        if statements.is_empty() {
            return Err("queries file contained no SQL statements".into());
        }
        return Ok(statements);
    }

    if std::io::stdin().is_terminal() {
        return Err("no SQL provided. Pass --query, --queries-file, or pipe SQL on stdin.".into());
    }

    let mut content = String::new();
    std::io::stdin().read_to_string(&mut content)?;
    if content.trim().is_empty() {
        return Err("no SQL received on stdin".into());
    }
    Ok(vec![content])
}

/// Split a ClickHouse query file on semicolons that occur outside lexical
/// constructs where semicolons are data. The returned statements omit the
/// delimiter and surrounding whitespace, and empty/comment-only segments are
/// ignored.
fn split_query_file(sql: &str) -> Vec<String> {
    let bytes = sql.as_bytes();
    let mut statements = Vec::new();
    let mut statement_start = 0;
    let mut has_sql = false;
    let mut pos = 0;
    let mut nesting = 0_usize;
    let mut is_insert = false;
    let mut is_explain = false;
    let mut insert_select = false;
    let mut expects_insert_target = false;
    let mut expects_insert_format = false;

    while pos < bytes.len() {
        match bytes[pos] {
            b'\xEF'
                if pos == statement_start && bytes.get(pos..pos + 3) == Some(b"\xEF\xBB\xBF") =>
            {
                pos += 3;
                statement_start = pos;
            }
            b'\xE2'
                if matches!(
                    bytes.get(pos..pos + 3),
                    Some(b"\xE2\x80\x98" | b"\xE2\x80\x9C")
                ) =>
            {
                has_sql = true;
                let closing_byte = bytes[pos + 2] + 1;
                pos = unicode_quoted_end(bytes, pos + 3, closing_byte);
                expects_insert_format = false;
            }
            b'\'' | b'"' | b'`' => {
                has_sql = true;
                pos = quoted_end(bytes, pos, bytes[pos]);
                if is_insert && nesting == 0 && expects_insert_target {
                    expects_insert_target = next_non_whitespace_is_dot(bytes, pos);
                }
                expects_insert_format = false;
            }
            b'-' if bytes.get(pos + 1) == Some(&b'-') => {
                pos = line_comment_end(bytes, pos + 2);
            }
            b'/' if bytes.get(pos + 1) == Some(&b'/') => {
                pos = line_comment_end(bytes, pos + 2);
            }
            b'/' if bytes.get(pos + 1) == Some(&b'*') => {
                pos = block_comment_end(bytes, pos + 2);
            }
            b'#' if matches!(bytes.get(pos + 1), Some(b' ' | b'!')) => {
                pos = line_comment_end(bytes, pos + 2);
            }
            b'$' => {
                has_sql = true;
                pos = match heredoc_end(bytes, pos) {
                    Some(Some(end)) => end,
                    Some(None) | None => pos + 1,
                };
                expects_insert_format = false;
            }
            b'(' | b'[' | b'{' => {
                has_sql = true;
                expects_insert_format = false;
                nesting += 1;
                pos += 1;
            }
            b')' | b']' | b'}' => {
                has_sql = true;
                expects_insert_format = false;
                nesting = nesting.saturating_sub(1);
                pos += 1;
            }
            b';' => {
                push_statement(sql, statement_start, pos, has_sql, &mut statements);
                pos += 1;
                statement_start = pos;
                has_sql = false;
                nesting = 0;
                is_insert = false;
                is_explain = false;
                insert_select = false;
                expects_insert_target = false;
                expects_insert_format = false;
            }
            byte if byte.is_ascii_alphanumeric() || byte == b'_' => {
                let word_start = pos;
                pos += 1;
                while matches!(bytes.get(pos), Some(b) if b.is_ascii_alphanumeric() || *b == b'_') {
                    pos += 1;
                }
                let word = &sql[word_start..pos];
                if !has_sql {
                    is_insert = word.eq_ignore_ascii_case("INSERT");
                    is_explain = word.eq_ignore_ascii_case("EXPLAIN");
                } else if is_explain
                    && !is_insert
                    && nesting == 0
                    && word.eq_ignore_ascii_case("INSERT")
                {
                    is_insert = true;
                } else if is_insert && nesting == 0 {
                    if expects_insert_target {
                        if !word.eq_ignore_ascii_case("TABLE")
                            && !word.eq_ignore_ascii_case("FUNCTION")
                        {
                            expects_insert_target = next_non_whitespace_is_dot(bytes, pos);
                        }
                    } else if expects_insert_format {
                        expects_insert_format = false;
                        if word.eq_ignore_ascii_case("SELECT") || word.eq_ignore_ascii_case("WITH")
                        {
                            insert_select = true;
                        } else if !word.eq_ignore_ascii_case("Values") {
                            let blank_line = find_blank_line(bytes, pos);
                            let raw_end = blank_line
                                .map(|(delimiter_start, _)| delimiter_start)
                                .unwrap_or(bytes.len());
                            push_raw_insert(sql, statement_start, raw_end, &mut statements);
                            if raw_end == bytes.len() {
                                return statements;
                            }
                            pos = raw_end + blank_line.unwrap().1;
                            statement_start = pos;
                            has_sql = false;
                            nesting = 0;
                            is_insert = false;
                            is_explain = false;
                            insert_select = false;
                            expects_insert_target = false;
                            continue;
                        }
                    } else if word.eq_ignore_ascii_case("INTO") {
                        expects_insert_target = true;
                    } else if word.eq_ignore_ascii_case("SELECT")
                        || word.eq_ignore_ascii_case("WITH")
                    {
                        // An INSERT ... SELECT has no inline data payload.
                        insert_select = true;
                    } else if !insert_select && word.eq_ignore_ascii_case("FORMAT") {
                        expects_insert_format = true;
                    }
                }
                has_sql = true;
            }
            byte => {
                if !byte.is_ascii_whitespace() {
                    has_sql = true;
                    expects_insert_format = false;
                }
                pos += 1;
            }
        }
    }

    push_statement(sql, statement_start, bytes.len(), has_sql, &mut statements);
    statements
}

fn push_statement(
    sql: &str,
    start: usize,
    end: usize,
    has_sql: bool,
    statements: &mut Vec<String>,
) {
    if has_sql {
        statements.push(sql[start..end].trim().to_string());
    }
}

/// Generic inline INSERT formats can contain arbitrary semicolons, so the
/// native ClickHouse client ends their data at a blank line instead. Preserve
/// their payload bytes apart from leading script whitespace.
fn push_raw_insert(sql: &str, start: usize, end: usize, statements: &mut Vec<String>) {
    statements.push(sql[start..end].trim_start().to_string());
}

fn quoted_end(bytes: &[u8], mut pos: usize, quote: u8) -> usize {
    pos += 1;
    while pos < bytes.len() {
        if bytes[pos] == b'\\' {
            pos = (pos + 2).min(bytes.len());
        } else if bytes[pos] == quote {
            pos += 1;
            if bytes.get(pos) == Some(&quote) {
                pos += 1;
            } else {
                break;
            }
        } else {
            pos += 1;
        }
    }
    pos
}

fn unicode_quoted_end(bytes: &[u8], mut pos: usize, closing_byte: u8) -> usize {
    while pos + 2 < bytes.len() {
        if bytes[pos..pos + 3] == [b'\xE2', b'\x80', closing_byte] {
            return pos + 3;
        }
        pos += 1;
    }
    bytes.len()
}

fn line_comment_end(bytes: &[u8], mut pos: usize) -> usize {
    while pos < bytes.len() && bytes[pos] != b'\n' {
        pos += 1;
    }
    pos
}

fn block_comment_end(bytes: &[u8], mut pos: usize) -> usize {
    let mut nesting = 1;
    while pos < bytes.len() {
        if bytes.get(pos..pos + 2) == Some(b"/*") {
            nesting += 1;
            pos += 2;
        } else if bytes.get(pos..pos + 2) == Some(b"*/") {
            nesting -= 1;
            pos += 2;
            if nesting == 0 {
                break;
            }
        } else {
            pos += 1;
        }
    }
    pos
}

/// Find the byte immediately after a ClickHouse `$tag$...$tag$` heredoc.
/// The tag follows ClickHouse's lexer rule: zero or more ASCII word
/// characters between the dollar signs. The outer option distinguishes a
/// dollar sign that is not an opening delimiter; the inner option is absent
/// when the apparent opener is only a `$`-containing bare word because there
/// is no later matching delimiter.
fn heredoc_end(bytes: &[u8], start: usize) -> Option<Option<usize>> {
    let relative_tag_end = bytes.get(start + 1..)?.iter().position(|b| *b == b'$')?;
    let tag_end = start + 1 + relative_tag_end;
    if !bytes[start + 1..tag_end]
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
    {
        return None;
    }

    let delimiter = &bytes[start..=tag_end];
    Some(
        find_bytes(&bytes[tag_end + 1..], delimiter)
            .map(|relative_end| tag_end + 1 + relative_end + delimiter.len()),
    )
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn find_blank_line(bytes: &[u8], mut pos: usize) -> Option<(usize, usize)> {
    while pos < bytes.len() {
        if bytes.get(pos..pos + 2) == Some(b"\n\n") {
            return Some((pos, 2));
        }
        if bytes.get(pos..pos + 4) == Some(b"\r\n\r\n") {
            return Some((pos, 4));
        }
        pos += 1;
    }
    None
}

fn next_non_whitespace_is_dot(bytes: &[u8], mut pos: usize) -> bool {
    while matches!(bytes.get(pos), Some(byte) if byte.is_ascii_whitespace()) {
        pos += 1;
    }
    bytes.get(pos) == Some(&b'.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_semicolon_delimited_statements_and_ignores_empty_segments() {
        assert_eq!(
            split_query_file(" ; SELECT 1;\n\nSELECT 2;; \n"),
            vec!["SELECT 1", "SELECT 2"]
        );
    }

    #[test]
    fn does_not_split_semicolons_in_quotes_or_heredocs() {
        let sql = r#"
            SELECT 'one;two', 'it''s;safe', 'backslash\';safe';
            SELECT "quoted;identifier", `backtick;identifier`;
            SELECT $tag$embedded; SQL and 'quotes'$tag$;
            SELECT $$an empty tag; works too$$;
        "#;

        assert_eq!(
            split_query_file(sql),
            vec![
                r#"SELECT 'one;two', 'it''s;safe', 'backslash\';safe'"#,
                r#"SELECT "quoted;identifier", `backtick;identifier`"#,
                "SELECT $tag$embedded; SQL and 'quotes'$tag$",
                "SELECT $$an empty tag; works too$$",
            ]
        );
    }

    #[test]
    fn a_dollar_wrapped_bare_word_without_heredoc_content_does_not_hide_delimiters() {
        assert_eq!(
            split_query_file("SELECT 1 AS $foo$; SELECT 2;"),
            vec!["SELECT 1 AS $foo$", "SELECT 2"]
        );
    }

    #[test]
    fn does_not_split_semicolons_in_unicode_quotes() {
        assert_eq!(
            split_query_file("SELECT ‘string; value’, “quoted; identifier”; SELECT 2;"),
            vec!["SELECT ‘string; value’, “quoted; identifier”", "SELECT 2"]
        );
    }

    #[test]
    fn strips_a_utf8_bom_before_splitting_the_first_statement() {
        assert_eq!(
            split_query_file("\u{feff}SELECT 'Здравствуйте; мир'; SELECT 2;"),
            vec!["SELECT 'Здравствуйте; мир'", "SELECT 2"]
        );
    }

    #[test]
    fn does_not_split_semicolons_in_clickhouse_comments() {
        let sql = r#"
            -- a SQL comment ;
            SELECT 1; // a C++ comment ;
            # a MySQL comment ;
            SELECT 2; #! a shebang-style comment ;
            /* outer ; /* nested ; */ still outer ; */ SELECT 3;
        "#;

        assert_eq!(
            split_query_file(sql),
            vec![
                "-- a SQL comment ;\n            SELECT 1",
                "// a C++ comment ;\n            # a MySQL comment ;\n            SELECT 2",
                "#! a shebang-style comment ;\n            /* outer ; /* nested ; */ still outer ; */ SELECT 3",
            ]
        );
    }

    #[test]
    fn ignores_a_file_containing_only_comments_and_delimiters() {
        assert!(
            split_query_file("-- comment;\n/* nested /* ; */ comment */ ; # final ;\n").is_empty()
        );
    }

    #[test]
    fn a_hash_without_comment_whitespace_does_not_hide_a_delimiter() {
        assert_eq!(
            split_query_file("SELECT #operator; SELECT 2;"),
            vec!["SELECT #operator", "SELECT 2"]
        );
    }

    #[test]
    fn generic_insert_format_data_ends_at_a_blank_line_not_a_semicolon() {
        let sql = "INSERT INTO events FORMAT CSV\n1,'a;b'\n2,'c;d'\n\nSELECT count() FROM events;";
        assert_eq!(
            split_query_file(sql),
            vec![
                "INSERT INTO events FORMAT CSV\n1,'a;b'\n2,'c;d'",
                "SELECT count() FROM events",
            ]
        );
    }

    #[test]
    fn generic_insert_format_accepts_a_windows_blank_line_delimiter() {
        let sql = "INSERT INTO events FORMAT CSV\r\n1,'a;b'\r\n\r\nSELECT 1;";
        assert_eq!(
            split_query_file(sql),
            vec!["INSERT INTO events FORMAT CSV\r\n1,'a;b'", "SELECT 1"]
        );
    }

    #[test]
    fn insert_values_uses_the_normal_semicolon_delimiter() {
        let sql = "INSERT INTO events FORMAT Values (1, 'a;b'); SELECT count() FROM events;";
        assert_eq!(
            split_query_file(sql),
            vec![
                "INSERT INTO events FORMAT Values (1, 'a;b')",
                "SELECT count() FROM events",
            ]
        );
    }

    #[test]
    fn format_used_as_an_insert_target_is_not_mistaken_for_a_format_clause() {
        assert_eq!(
            split_query_file("INSERT INTO db.format SELECT 1; SELECT 2;"),
            vec!["INSERT INTO db.format SELECT 1", "SELECT 2"]
        );
        assert_eq!(
            split_query_file("INSERT INTO TABLE format SELECT 1; SELECT 2;"),
            vec!["INSERT INTO TABLE format SELECT 1", "SELECT 2"]
        );
    }

    #[test]
    fn format_used_in_an_insert_select_cte_is_not_mistaken_for_inline_data() {
        assert_eq!(
            split_query_file("INSERT INTO dst WITH 1 AS format SELECT format; SELECT 2;"),
            vec!["INSERT INTO dst WITH 1 AS format SELECT format", "SELECT 2"]
        );
    }

    #[test]
    fn explain_insert_format_data_uses_the_blank_line_boundary() {
        let sql = "EXPLAIN INSERT INTO events FORMAT CSV\n1,'a;b'\n\nSELECT 2;";
        assert_eq!(
            split_query_file(sql),
            vec!["EXPLAIN INSERT INTO events FORMAT CSV\n1,'a;b'", "SELECT 2",]
        );
    }
}
