//! `search`: find commands and flags by keyword (#1161).
//!
//! The index is built at run time from the clap tree, so it can never go
//! stale. It reads the undecorated tree (`Cli::augment_args`, not
//! `Cli::command`), so the permission lines `cloud::permissions` appends to
//! Cloud help are not indexed as agent context.

use std::fmt;

use clap::Command;
use serde::Serialize;

use crate::cli::{Cli, SearchArgs, help_order};
use crate::error::Result;

const PATH_WEIGHT: u32 = 8;
const FLAG_NAME_WEIGHT: u32 = 5;
const ABOUT_WEIGHT: u32 = 4;
const FLAG_HELP_WEIGHT: u32 = 2;
const CONTEXT_WEIGHT: u32 = 1;
/// Matching flags shown under each command.
const MAX_FLAGS: usize = 3;
/// A query word this long also matches words it begins ("time" → "timeout").
const MIN_PREFIX_LEN: usize = 3;
const CONTEXT_HEADER: &str = "CONTEXT FOR AGENTS:";

/// Console and common vocabulary the CLI words differently: each phrase in
/// a query is replaced by the CLI's word. Every target must be a word in the
/// index (tested).
const SYNONYMS: &[(&[&str], &str)] = &[
    // "IP access list" in the console is `--ip-allow` here.
    (&["access", "list"], "allow"),
    (&["allowlist"], "allow"),
    (&["whitelist"], "allow"),
];

/// The query's distinct words, with synonym phrases replaced.
fn query_terms(query: &[String]) -> Vec<String> {
    let words: Vec<String> = query.iter().flat_map(|word| tokenize(word)).collect();
    let mut terms = Vec::new();
    let mut rest = words.as_slice();
    while let Some(word) = rest.first() {
        let synonym = SYNONYMS.iter().find(|(phrase, _)| {
            rest.len() >= phrase.len() && rest.iter().zip(phrase.iter()).all(|(a, b)| a == b)
        });
        match synonym {
            Some((phrase, target)) => {
                terms.push((*target).to_owned());
                rest = &rest[phrase.len()..];
            }
            None => {
                terms.push(word.clone());
                rest = &rest[1..];
            }
        }
    }
    terms.sort();
    terms.dedup();
    terms
}

pub fn run(args: SearchArgs, json: bool) -> Result<()> {
    let root = <Cli as clap::Args>::augment_args(Command::new("clickhousectl"));
    let output = search(&index(&root), &args.terms, args.limit);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&output).expect("JSON serialization failed")
        );
    } else {
        print!("{output}");
    }
    Ok(())
}

/// Lowercase words: runs of ASCII letters and digits.
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn term_matches(term: &str, token: &str) -> bool {
    token == term || (term.len() >= MIN_PREFIX_LEN && token.starts_with(term))
}

fn field_matches(term: &str, tokens: &[String]) -> bool {
    tokens.iter().any(|token| term_matches(term, token))
}

struct Flag {
    flag: String,
    help: String,
    name_tokens: Vec<String>,
    help_tokens: Vec<String>,
}

impl Flag {
    fn score(&self, terms: &[String]) -> u32 {
        terms
            .iter()
            .map(|term| {
                if field_matches(term, &self.name_tokens) {
                    FLAG_NAME_WEIGHT
                } else if field_matches(term, &self.help_tokens) {
                    FLAG_HELP_WEIGHT
                } else {
                    0
                }
            })
            .sum()
    }
}

struct Entry {
    path: Vec<String>,
    about: String,
    path_tokens: Vec<String>,
    about_tokens: Vec<String>,
    context_tokens: Vec<String>,
    flags: Vec<Flag>,
}

impl Entry {
    fn command(&self) -> String {
        self.path.join(" ")
    }

    /// Sum over query words of the heaviest field each one matches.
    fn score(&self, terms: &[String]) -> u32 {
        terms
            .iter()
            .map(|term| {
                [
                    (PATH_WEIGHT, field_matches(term, &self.path_tokens)),
                    (
                        FLAG_NAME_WEIGHT,
                        self.flags
                            .iter()
                            .any(|flag| field_matches(term, &flag.name_tokens)),
                    ),
                    (ABOUT_WEIGHT, field_matches(term, &self.about_tokens)),
                    (
                        FLAG_HELP_WEIGHT,
                        self.flags
                            .iter()
                            .any(|flag| field_matches(term, &flag.help_tokens)),
                    ),
                    (CONTEXT_WEIGHT, field_matches(term, &self.context_tokens)),
                ]
                .into_iter()
                .find_map(|(weight, matched)| matched.then_some(weight))
                .unwrap_or(0)
            })
            .sum()
    }
}

/// Every visible executable command under `root`, with its visible
/// command-specific flags. Grouping commands (those that require a
/// subcommand), hidden commands and their subtrees, hidden flags and the
/// shared flag block (`help_order` ranks) are left out.
fn index(root: &Command) -> Vec<Entry> {
    fn walk(command: &Command, path: &mut Vec<String>, entries: &mut Vec<Entry>) {
        for child in command.get_subcommands() {
            if child.is_hide_set() {
                continue;
            }
            path.push(child.get_name().to_owned());
            if !child.is_subcommand_required_set() {
                entries.push(entry(child, path.clone()));
            }
            walk(child, path, entries);
            path.pop();
        }
    }
    let mut entries = Vec::new();
    walk(root, &mut Vec::new(), &mut entries);
    entries
}

fn entry(command: &Command, path: Vec<String>) -> Entry {
    let about = command
        .get_about()
        .map(ToString::to_string)
        .unwrap_or_default();
    let context = command
        .get_after_help()
        .map(ToString::to_string)
        .unwrap_or_default();
    let context = context.trim_start().trim_start_matches(CONTEXT_HEADER);
    let flags = command
        .get_arguments()
        .filter(|arg| !arg.is_hide_set() && arg.get_display_order() < help_order::ORG_ID)
        .filter_map(|arg| {
            let long = arg.get_long()?;
            let help = arg.get_help().map(ToString::to_string).unwrap_or_default();
            Some(Flag {
                flag: format!("--{long}"),
                name_tokens: tokenize(long),
                help_tokens: tokenize(&help),
                help,
            })
        })
        .collect();
    Entry {
        path_tokens: path.iter().flat_map(|segment| tokenize(segment)).collect(),
        about_tokens: tokenize(&about),
        context_tokens: tokenize(context),
        about,
        path,
        flags,
    }
}

#[derive(Debug, Serialize)]
pub struct SearchOutput {
    results: Vec<SearchResult>,
}

#[derive(Debug, Serialize)]
struct SearchResult {
    command: String,
    about: String,
    flags: Vec<FlagMatch>,
}

#[derive(Debug, Serialize)]
struct FlagMatch {
    flag: String,
    help: String,
}

/// The `limit` best commands for the query, best first. Ties go to the
/// shorter command path, then to the command `--help` lists first: help
/// order puts the main resources before secondary ones.
fn search(entries: &[Entry], query: &[String], limit: usize) -> SearchOutput {
    let terms = query_terms(query);
    let mut scored: Vec<(u32, &Entry)> = entries
        .iter()
        .map(|entry| (entry.score(&terms), entry))
        .filter(|(score, _)| *score > 0)
        .collect();
    // Stable: `entries` is in help order.
    scored.sort_by(|(a_score, a), (b_score, b)| {
        b_score
            .cmp(a_score)
            .then_with(|| a.path.len().cmp(&b.path.len()))
    });
    let results = scored
        .into_iter()
        .take(limit)
        .map(|(_, entry)| {
            let mut flags: Vec<(u32, &Flag)> = entry
                .flags
                .iter()
                .map(|flag| (flag.score(&terms), flag))
                .filter(|(score, _)| *score > 0)
                .collect();
            // Stable: equal scores keep declaration order.
            flags.sort_by(|(a, _), (b, _)| b.cmp(a));
            SearchResult {
                command: entry.command(),
                about: entry.about.clone(),
                flags: flags
                    .into_iter()
                    .take(MAX_FLAGS)
                    .map(|(_, flag)| FlagMatch {
                        flag: flag.flag.clone(),
                        help: flag.help.clone(),
                    })
                    .collect(),
            }
        })
        .collect();
    SearchOutput { results }
}

impl fmt::Display for SearchOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.results.is_empty() {
            return writeln!(
                f,
                "No matching commands. Try other words, or `clickhousectl --help`."
            );
        }
        for result in &self.results {
            writeln!(f, "{}  {}", result.command, result.about)?;
            let width = result
                .flags
                .iter()
                .map(|flag| flag.flag.len())
                .max()
                .unwrap_or(0);
            for flag in &result.flags {
                writeln!(f, "    {:width$}  {}", flag.flag, flag.help)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn real_index() -> Vec<Entry> {
        index(&<Cli as clap::Args>::augment_args(Command::new(
            "clickhousectl",
        )))
    }

    fn query(words: &str) -> Vec<String> {
        words.split_whitespace().map(str::to_owned).collect()
    }

    fn commands(output: &SearchOutput) -> Vec<&str> {
        output
            .results
            .iter()
            .map(|result| result.command.as_str())
            .collect()
    }

    #[test]
    fn tokenize_lowercases_and_splits_on_non_alphanumerics() {
        assert_eq!(
            tokenize("--add-ip-allow IP/CIDR, x_y"),
            ["add", "ip", "allow", "ip", "cidr", "x", "y"]
        );
        assert!(tokenize(" -- / ").is_empty());
    }

    #[test]
    fn short_words_match_whole_words_and_longer_ones_also_prefixes() {
        assert!(term_matches("ip", "ip"));
        assert!(!term_matches("ip", "ipv6"));
        assert!(term_matches("time", "timeout"));
        assert!(!term_matches("timeout", "time"));
        assert!(!term_matches("out", "timeout"));
    }

    /// Only visible executable commands are indexed, and only their own
    /// visible flags: never a hidden command or flag, a grouping command, or
    /// the shared flag block.
    #[test]
    fn index_skips_hidden_grouping_and_shared_entries() {
        let root = <Cli as clap::Args>::augment_args(Command::new("clickhousectl"));
        let entries = index(&root);
        let indexed: BTreeSet<String> = entries.iter().map(Entry::command).collect();
        assert!(indexed.contains("cloud service update"), "{indexed:?}");
        assert!(indexed.contains("search"));

        fn walk(
            command: &Command,
            path: &str,
            hidden: bool,
            check: &mut dyn FnMut(&Command, &str, bool),
        ) {
            for child in command.get_subcommands() {
                let child_path = format!("{path} {}", child.get_name());
                let child_path = child_path.trim_start().to_owned();
                let hidden = hidden || child.is_hide_set();
                check(child, &child_path, hidden);
                walk(child, &child_path, hidden, check);
            }
        }
        let mut hidden_seen = 0;
        walk(&root, "", false, &mut |command, path, hidden| {
            let expected = !hidden && !command.is_subcommand_required_set();
            assert_eq!(indexed.contains(path), expected, "{path}");
            hidden_seen += usize::from(hidden);
        });
        if cfg!(feature = "telemetry") {
            assert!(hidden_seen > 0, "the tree has a hidden command");
        }

        let shared = [
            "--org-id",
            "--org-name",
            "--api-key",
            "--api-secret",
            "--url",
            "--json",
            "--debug",
            "--help",
        ];
        for entry in &entries {
            let command = entry
                .path
                .iter()
                .try_fold(&root, |command, name| command.find_subcommand(name))
                .unwrap();
            for flag in &entry.flags {
                assert!(
                    !shared.contains(&flag.flag.as_str()),
                    "{}: {}",
                    entry.command(),
                    flag.flag
                );
                let arg = command
                    .get_arguments()
                    .find(|arg| {
                        arg.get_long().map(|long| format!("--{long}")) == Some(flag.flag.clone())
                    })
                    .unwrap();
                assert!(!arg.is_hide_set(), "{}: {}", entry.command(), flag.flag);
            }
        }
        // `local client` keeps a hidden `--name` alias for its positional.
        let client = entries
            .iter()
            .find(|entry| entry.command() == "local client")
            .unwrap();
        assert!(client.flags.iter().all(|flag| flag.flag != "--name"));
    }

    #[test]
    fn equal_scores_rank_shorter_paths_then_help_order() {
        let root = Command::new("root")
            .subcommand(
                Command::new("group")
                    .subcommand_required(true)
                    .subcommand(Command::new("leaf").about("Widget tool")),
            )
            .subcommand(Command::new("zeta").about("Widget tool"))
            .subcommand(Command::new("alpha").about("Widget tool"));
        let entries = index(&root);
        for _ in 0..3 {
            let output = search(&entries, &query("widget"), 5);
            assert_eq!(commands(&output), ["zeta", "alpha", "group leaf"]);
        }
        assert_eq!(
            commands(&search(&entries, &query("widget"), 2)),
            ["zeta", "alpha"]
        );
    }

    #[test]
    fn a_path_match_outranks_flag_about_help_and_context_matches() {
        let root = Command::new("root")
            .subcommand(
                Command::new("ctx")
                    .about("Other")
                    .after_help("CONTEXT FOR AGENTS:\n  mentions gadget"),
            )
            .subcommand(
                Command::new("help-text")
                    .about("Other")
                    .arg(clap::Arg::new("x").long("x").help("A gadget")),
            )
            .subcommand(Command::new("about").about("Manage gadget"))
            .subcommand(
                Command::new("flag")
                    .about("Other")
                    .arg(clap::Arg::new("gadget-id").long("gadget-id").help("Id")),
            )
            .subcommand(Command::new("gadget").about("Other"));
        let output = search(&index(&root), &query("gadget"), 10);
        assert_eq!(
            commands(&output),
            ["gadget", "flag", "about", "help-text", "ctx"]
        );
        // The context header itself is never a match.
        assert!(commands(&search(&index(&root), &query("agents"), 10)).is_empty());
    }

    #[test]
    fn ip_allow_list_finds_service_update_first_and_create_nearby() {
        let output = search(&real_index(), &query("ip allow list"), 5);
        let found = commands(&output);
        assert_eq!(found.first(), Some(&"cloud service update"), "{found:?}");
        assert!(found.contains(&"cloud service create"), "{found:?}");
        let flags: Vec<&str> = output.results[0]
            .flags
            .iter()
            .map(|flag| flag.flag.as_str())
            .collect();
        assert!(flags.contains(&"--add-ip-allow"), "{flags:?}");
        assert!(flags.contains(&"--remove-ip-allow"), "{flags:?}");
    }

    #[test]
    fn ip_access_list_reaches_the_ip_allow_flags() {
        let output = search(&real_index(), &query("IP access list"), 5);
        let found = commands(&output);
        for command in [
            "cloud service create",
            "cloud service update",
            "cloud postgres update",
        ] {
            assert!(found.contains(&command), "{found:?}");
        }
    }

    #[test]
    fn synonym_phrases_are_found_and_targets_are_indexed_words() {
        let words: BTreeSet<String> = real_index()
            .iter()
            .flat_map(|entry| {
                entry
                    .flags
                    .iter()
                    .flat_map(|flag| flag.name_tokens.iter().chain(&flag.help_tokens))
                    .chain(&entry.path_tokens)
                    .chain(&entry.about_tokens)
                    .chain(&entry.context_tokens)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .collect();
        for (phrase, target) in SYNONYMS {
            assert!(words.contains(*target), "{target}");
            let query: Vec<String> = phrase.iter().map(|word| word.to_uppercase()).collect();
            assert!(
                query_terms(&query).iter().any(|term| term == target),
                "{phrase:?}"
            );
        }
        // A phrase only counts when its words are adjacent and in order.
        assert!(!query_terms(&query("list access")).contains(&"allow".to_owned()));
    }

    #[test]
    fn idle_timeout_finds_service_create_and_scale() {
        let output = search(&real_index(), &query("idle timeout"), 5);
        let found = commands(&output);
        for command in ["cloud service create", "cloud service scale"] {
            assert!(found.contains(&command), "{found:?}");
        }
    }

    #[test]
    fn no_match_returns_no_results() {
        let output = search(&real_index(), &query("zzqqxx"), 5);
        assert!(output.results.is_empty());
        let output = search(&real_index(), &query("--"), 5);
        assert!(output.results.is_empty());
    }

    #[test]
    fn results_respect_the_limit_and_flags_are_capped() {
        let output = search(&real_index(), &query("service"), 50);
        assert!(output.results.len() > 5);
        assert!(output.results.len() <= 50);
        assert!(
            output
                .results
                .iter()
                .all(|result| result.flags.len() <= MAX_FLAGS)
        );
        assert_eq!(search(&real_index(), &query("service"), 1).results.len(), 1);
    }

    #[test]
    fn json_has_only_command_about_and_flags() {
        let output = search(&real_index(), &query("ip allow"), 5);
        let value = serde_json::to_value(&output).unwrap();
        let object = value.as_object().unwrap();
        assert_eq!(object.keys().collect::<Vec<_>>(), ["results"]);
        let results = object["results"].as_array().unwrap();
        assert!(!results.is_empty());
        for result in results {
            let result = result.as_object().unwrap();
            assert_eq!(
                result.keys().map(String::as_str).collect::<BTreeSet<_>>(),
                BTreeSet::from(["command", "about", "flags"])
            );
            assert!(result["command"].is_string() && result["about"].is_string());
            for flag in result["flags"].as_array().unwrap() {
                let flag = flag.as_object().unwrap();
                assert_eq!(
                    flag.keys().map(String::as_str).collect::<BTreeSet<_>>(),
                    BTreeSet::from(["flag", "help"])
                );
                assert!(flag["flag"].as_str().unwrap().starts_with("--"));
            }
        }
        let empty = serde_json::to_value(search(&real_index(), &query("zzqqxx"), 5)).unwrap();
        assert_eq!(empty, serde_json::json!({ "results": [] }));
    }
}
