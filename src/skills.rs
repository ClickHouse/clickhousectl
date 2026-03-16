use crate::cli::SkillsArgs;
use crate::error::{Error, Result};
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tar::Archive;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Copy, Eq, PartialEq)]
enum InstallScope {
    Project,
    Global,
}

struct AgentSpec {
    key: &'static str,
    name: &'static str,
    label: &'static str,
    install_dir: &'static str,
}

struct InstallSummary {
    created_files: usize,
    updated_files: usize,
    unchanged_files: usize,
}

struct SkillFile {
    skill_slug: String,
    relative_path: PathBuf,
    source_path: PathBuf,
}

const AGENT_SKILLS_REPO: &str = "https://github.com/ClickHouse/agent-skills";
const AGENT_SKILLS_ARCHIVE_URL: &str =
    "https://codeload.github.com/ClickHouse/agent-skills/tar.gz/refs/heads/main";
const UNIVERSAL_AGENT_KEY: &str = "agents";
const UNIVERSAL_COVERAGE: &[&str] = &["Amp", "Cline", "Codex", "Cursor", "OpenCode"];

const SUPPORTED_AGENTS: &[AgentSpec] = &[
    AgentSpec {
        key: "agents",
        name: "Amp",
        label: ".agents",
        install_dir: ".agents/skills",
    },
    AgentSpec {
        key: "claude",
        name: "Claude Code",
        label: ".claude",
        install_dir: ".claude/skills",
    },
    AgentSpec {
        key: "codex",
        name: "Codex",
        label: ".codex",
        install_dir: ".codex/skills",
    },
    AgentSpec {
        key: "cursor",
        name: "Cursor",
        label: ".cursor",
        install_dir: ".cursor/skills",
    },
    AgentSpec {
        key: "opencode",
        name: "OpenCode",
        label: ".opencode",
        install_dir: ".opencode/skills",
    },
    AgentSpec {
        key: "agent",
        name: "Antigravity",
        label: ".agent",
        install_dir: ".agent/skills",
    },
    AgentSpec {
        key: "roo",
        name: "Roo",
        label: ".roo",
        install_dir: ".roo/skills",
    },
    AgentSpec {
        key: "trae",
        name: "Trae",
        label: ".trae",
        install_dir: ".trae/skills",
    },
    AgentSpec {
        key: "windsurf",
        name: "Windsurf",
        label: ".windsurf",
        install_dir: ".windsurf/skills",
    },
    AgentSpec {
        key: "zencoder",
        name: "Zencoder",
        label: ".zencoder",
        install_dir: ".zencoder/skills",
    },
    AgentSpec {
        key: "neovate",
        name: "Neovate",
        label: ".neovate",
        install_dir: ".neovate/skills",
    },
    AgentSpec {
        key: "pochi",
        name: "Pochi",
        label: ".pochi",
        install_dir: ".pochi/skills",
    },
    AgentSpec {
        key: "adal",
        name: "Adal",
        label: ".adal",
        install_dir: ".adal/skills",
    },
    AgentSpec {
        key: "openclaw",
        name: "OpenClaw",
        label: ".openclaw",
        install_dir: ".openclaw/skills",
    },
    AgentSpec {
        key: "cline",
        name: "Cline",
        label: ".cline",
        install_dir: ".cline/skills",
    },
    AgentSpec {
        key: "command-code",
        name: "Command Code",
        label: ".command-code",
        install_dir: ".command-code/skills",
    },
    AgentSpec {
        key: "kiro-cli",
        name: "Kiro CLI",
        label: ".kiro",
        install_dir: ".kiro/skills",
    },
];

pub async fn install(args: SkillsArgs) -> Result<()> {
    let home = home_dir()?;
    let scope = resolve_scope(&args)?;
    let root = scope_root(scope)?;
    let detected = detect_agents(&home);
    let selected = resolve_selection(&root, &detected, &args)?;

    println!(
        "Downloading ClickHouse agent skills from {}...",
        AGENT_SKILLS_REPO
    );
    let archive = download_agent_skills_archive().await?;
    let extracted = extract_skills_from_tarball(&archive.path)?;
    let skill_files = collect_skill_files(&extracted.path)?;

    if skill_files.is_empty() {
        return Err(Error::Skills(format!(
            "No skills were found in the official repo archive from {}.",
            AGENT_SKILLS_REPO
        )));
    }

    let skill_names = installed_skill_names(&skill_files);
    println!("Installing skills: {}", skill_names.join(", "));

    for agent in selected {
        let summary = install_into_agent(&root, agent, &skill_files)?;
        let skill_dir = root.join(agent.install_dir);
        println!(
            "  {} -> {} (created {}, updated {}, unchanged {})",
            agent.key,
            skill_dir.display(),
            summary.created_files,
            summary.updated_files,
            summary.unchanged_files
        );
    }

    Ok(())
}

struct TempArtifact {
    path: PathBuf,
    is_dir: bool,
}

impl Drop for TempArtifact {
    fn drop(&mut self) {
        let _ = if self.is_dir {
            fs::remove_dir_all(&self.path)
        } else {
            fs::remove_file(&self.path)
        };
    }
}

fn create_temp_artifact(prefix: &str, suffix: &str, is_dir: bool) -> Result<TempArtifact> {
    let base = std::env::temp_dir();
    let pid = std::process::id();

    for attempt in 0..100u32 {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| Error::Skills(format!("Failed to get system time: {err}")))?
            .as_nanos();
        let name = format!("{prefix}-{pid}-{timestamp}-{attempt}{suffix}");
        let path = base.join(name);

        if is_dir {
            match fs::create_dir(&path) {
                Ok(()) => return Ok(TempArtifact { path, is_dir }),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(err.into()),
            }
        } else {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => return Ok(TempArtifact { path, is_dir }),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(err.into()),
            }
        }
    }

    Err(Error::Skills(format!(
        "Failed to create a temporary {} after multiple attempts.",
        if is_dir { "directory" } else { "file" }
    )))
}

async fn download_agent_skills_archive() -> Result<TempArtifact> {
    let response = reqwest::get(AGENT_SKILLS_ARCHIVE_URL).await?;
    if !response.status().is_success() {
        return Err(Error::Skills(format!(
            "Failed to download {}: HTTP {}",
            AGENT_SKILLS_ARCHIVE_URL,
            response.status()
        )));
    }

    let temp_file = create_temp_artifact("clickhouse-agent-skills", ".tar.gz", false)?;
    let mut file = tokio::fs::File::create(&temp_file.path).await?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk?).await?;
    }
    file.flush().await?;

    Ok(temp_file)
}

fn extract_skills_from_tarball(archive_path: &Path) -> Result<TempArtifact> {
    let archive_file = File::open(archive_path)?;
    let decoder = GzDecoder::new(archive_file);
    let mut archive = Archive::new(decoder);
    let temp_dir = create_temp_artifact("clickhouse-agent-skills", "", true)?;

    for entry in archive.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }

        let path = entry.path()?.into_owned();
        let Some((skill_slug, relative_path)) = parse_skill_archive_path(&path) else {
            continue;
        };

        let output_path = temp_dir.path.join(skill_slug).join(relative_path);
        ensure_within(&temp_dir.path, &output_path)?;
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(output_path)?;
        io::copy(&mut entry, &mut output)?;
    }

    Ok(temp_dir)
}

fn parse_skill_archive_path(path: &Path) -> Option<(String, PathBuf)> {
    let mut components = path.components();
    components.next()?;

    match components.next()? {
        Component::Normal(part) if part == "skills" => {}
        _ => return None,
    }

    let skill_slug = match components.next()? {
        Component::Normal(part) => part.to_str()?.to_string(),
        _ => return None,
    };

    let mut relative_path = PathBuf::new();
    for component in components {
        match component {
            Component::Normal(part) => relative_path.push(part),
            _ => return None,
        }
    }

    if relative_path.as_os_str().is_empty() {
        return None;
    }

    Some((skill_slug, relative_path))
}

fn ensure_within(base: &Path, candidate: &Path) -> Result<()> {
    candidate
        .strip_prefix(base)
        .map(|_| ())
        .map_err(|_| Error::Skills(format!("Refusing to write outside {}", base.display())))
}

fn installed_skill_names(skill_files: &[SkillFile]) -> Vec<String> {
    let mut names = skill_files
        .iter()
        .map(|file| file.skill_slug.clone())
        .collect::<Vec<_>>();
    names.dedup();
    names
}

fn collect_skill_files(extracted_root: &Path) -> Result<Vec<SkillFile>> {
    let mut skill_files = Vec::new();

    for skill_dir in fs::read_dir(extracted_root)? {
        let skill_dir = skill_dir?;
        if !skill_dir.file_type()?.is_dir() {
            continue;
        }

        let skill_slug = skill_dir.file_name().to_string_lossy().into_owned();
        collect_skill_files_recursive(
            &skill_dir.path(),
            &skill_dir.path(),
            &skill_slug,
            &mut skill_files,
        )?;
    }

    skill_files.sort_by(|left, right| {
        left.skill_slug
            .cmp(&right.skill_slug)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });

    Ok(skill_files)
}

fn collect_skill_files_recursive(
    skill_root: &Path,
    current_dir: &Path,
    skill_slug: &str,
    skill_files: &mut Vec<SkillFile>,
) -> Result<()> {
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            collect_skill_files_recursive(skill_root, &path, skill_slug, skill_files)?;
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let relative_path = path
            .strip_prefix(skill_root)
            .map_err(|err| Error::Skills(format!("Failed to compute skill path: {err}")))?
            .to_path_buf();

        skill_files.push(SkillFile {
            skill_slug: skill_slug.to_string(),
            relative_path,
            source_path: path,
        });
    }

    Ok(())
}

fn resolve_selection<'a>(
    _install_root: &Path,
    detected: &'a [&'static AgentSpec],
    args: &SkillsArgs,
) -> Result<Vec<&'static AgentSpec>> {
    if args.all {
        return Ok(SUPPORTED_AGENTS.iter().collect());
    }

    if args.detected_only {
        let mut selected = vec![universal_agent()];
        selected.extend(
            detected
                .iter()
                .copied()
                .filter(|agent| agent.key != UNIVERSAL_AGENT_KEY),
        );
        return Ok(selected);
    }

    if !args.agents.is_empty() {
        let mut selected = vec![universal_agent()];
        for raw_agent in &args.agents {
            let agent = find_agent(raw_agent).ok_or_else(|| {
                Error::Skills(format!(
                    "Unsupported agent '{}'. Supported agents: {}",
                    raw_agent,
                    supported_agent_list()
                ))
            })?;
            if !selected
                .iter()
                .any(|existing: &&AgentSpec| existing.key == agent.key)
            {
                selected.push(agent);
            }
        }
        return Ok(selected);
    }

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(Error::Skills(
            "Interactive selection requires a TTY. Use --all or --agent <name> in non-interactive environments.".into(),
        ));
    }

    interactive_select(detected)
}

fn detect_agents(root: &Path) -> Vec<&'static AgentSpec> {
    SUPPORTED_AGENTS
        .iter()
        .filter(|agent| root.join(agent.label).is_dir())
        .collect()
}

fn interactive_select(detected: &[&'static AgentSpec]) -> Result<Vec<&'static AgentSpec>> {
    let mut stdout = io::stdout();
    let _terminal_ui = TerminalUi::new(&mut stdout)?;
    let agents = ordered_additional_agents(detected);
    let mut selected = agents
        .iter()
        .map(|agent| {
            detected
                .iter()
                .any(|detected_agent| detected_agent.key == agent.key)
        })
        .collect::<Vec<_>>();
    let mut cursor = 0usize;

    loop {
        render_picker(&mut stdout, detected, &agents, &selected, cursor)?;

        match read_key()? {
            Key::Up => {
                cursor = cursor.saturating_sub(1);
            }
            Key::Down => {
                if cursor + 1 < agents.len() {
                    cursor += 1;
                }
            }
            Key::Toggle => {
                selected[cursor] = !selected[cursor];
            }
            Key::Confirm => {
                let mut selected_agents = vec![universal_agent()];
                selected_agents.extend(
                    agents
                        .iter()
                        .enumerate()
                        .filter_map(|(index, agent)| selected[index].then_some(*agent))
                        .collect::<Vec<_>>(),
                );
                return Ok(selected_agents);
            }
            Key::Cancel => {
                return Err(Error::Skills("Installation cancelled.".into()));
            }
        }
    }
}

fn render_picker(
    stdout: &mut io::Stdout,
    detected: &[&'static AgentSpec],
    agents: &[&'static AgentSpec],
    selected: &[bool],
    cursor: usize,
) -> io::Result<()> {
    write!(stdout, "\x1b[2J\x1b[H")?;
    write_line(
        stdout,
        &format!(
            "{} installed agents detected in home directory",
            detected.len()
        ),
    )?;
    write_line(stdout, "Which agents do you want to install to?")?;
    write_line(stdout, "")?;
    write_line(stdout, "-- Universal (.agents/skills) -- always included")?;
    for name in UNIVERSAL_COVERAGE {
        write_line(stdout, &format!("  • {}", name))?;
    }
    write_line(stdout, "")?;
    write_line(stdout, "-- Additional agents --")?;
    write_line(stdout, "  Up/Down move, Space select, Enter confirm")?;
    write_line(stdout, "")?;

    for (index, agent) in agents.iter().enumerate() {
        let pointer = if index == cursor { ">" } else { " " };
        let checkbox = if selected[index] { "●" } else { "○" };
        write_line(
            stdout,
            &format!(
                "{} {} {} ({})",
                pointer, checkbox, agent.name, agent.install_dir
            ),
        )?;
    }

    write_line(stdout, "")?;
    write_line(
        stdout,
        &format!(
            "Selected: {}",
            std::iter::once(universal_agent().key)
                .chain(
                    agents
                        .iter()
                        .zip(selected.iter())
                        .filter_map(|(agent, is_selected)| is_selected.then_some(agent.key)),
                )
                .collect::<Vec<_>>()
                .join(", ")
        ),
    )?;
    stdout.flush()
}

fn ordered_additional_agents(detected: &[&'static AgentSpec]) -> Vec<&'static AgentSpec> {
    let mut ordered = detected
        .iter()
        .copied()
        .filter(|agent| agent.key != UNIVERSAL_AGENT_KEY)
        .collect::<Vec<_>>();
    for agent in additional_agents() {
        if !ordered.iter().any(|existing| existing.key == agent.key) {
            ordered.push(agent);
        }
    }
    ordered
}

fn additional_agents() -> impl Iterator<Item = &'static AgentSpec> {
    SUPPORTED_AGENTS
        .iter()
        .filter(|agent| agent.key != UNIVERSAL_AGENT_KEY)
}

fn universal_agent() -> &'static AgentSpec {
    SUPPORTED_AGENTS
        .iter()
        .find(|agent| agent.key == UNIVERSAL_AGENT_KEY)
        .expect("universal .agents support must exist")
}

fn write_line(stdout: &mut io::Stdout, line: &str) -> io::Result<()> {
    write!(stdout, "{line}\r\n")
}

fn install_into_agent(
    root: &Path,
    agent: &'static AgentSpec,
    skill_files: &[SkillFile],
) -> Result<InstallSummary> {
    let mut summary = InstallSummary {
        created_files: 0,
        updated_files: 0,
        unchanged_files: 0,
    };

    for file in skill_files {
        let install_root = root.join(agent.install_dir);
        let output_path = install_root
            .join(&file.skill_slug)
            .join(&file.relative_path);
        ensure_within(&install_root, &output_path)?;
        let source_contents = fs::read(&file.source_path)?;

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        match std::fs::read(&output_path) {
            Ok(existing) if existing == source_contents => {
                summary.unchanged_files += 1;
            }
            Ok(_) => {
                std::fs::copy(&file.source_path, &output_path)?;
                summary.updated_files += 1;
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                std::fs::copy(&file.source_path, &output_path)?;
                summary.created_files += 1;
            }
            Err(err) => return Err(err.into()),
        }
    }

    Ok(summary)
}

fn find_agent(name: &str) -> Option<&'static AgentSpec> {
    let normalized = name.trim().to_ascii_lowercase();
    let normalized = normalized.trim_start_matches('.').to_string();
    SUPPORTED_AGENTS
        .iter()
        .find(|agent| agent.key == normalized)
}

fn supported_agent_list() -> String {
    SUPPORTED_AGENTS
        .iter()
        .map(|agent| agent.key)
        .collect::<Vec<_>>()
        .join(", ")
}

fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| {
        Error::Skills("Could not determine the current user's home directory.".into())
    })
}

fn scope_root(scope: InstallScope) -> Result<PathBuf> {
    match scope {
        InstallScope::Project => std::env::current_dir().map_err(Into::into),
        InstallScope::Global => home_dir(),
    }
}

fn resolve_scope(args: &SkillsArgs) -> Result<InstallScope> {
    if args.global {
        return Ok(InstallScope::Global);
    }

    if args.all || args.detected_only || !args.agents.is_empty() {
        return Ok(InstallScope::Project);
    }

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(InstallScope::Project);
    }

    interactive_scope_select()
}

fn interactive_scope_select() -> Result<InstallScope> {
    let mut stdout = io::stdout();
    let _terminal_ui = TerminalUi::new(&mut stdout)?;
    let mut cursor = 0usize;
    let scopes = [
        (
            InstallScope::Project,
            "Project",
            "Install in current directory (committed with your project)",
        ),
        (
            InstallScope::Global,
            "Global",
            "Install in your home directory agent configs",
        ),
    ];

    loop {
        write!(stdout, "\x1b[2J\x1b[H")?;
        write_line(&mut stdout, "Installation scope")?;
        write_line(&mut stdout, "")?;

        for (index, (_, label, description)) in scopes.iter().enumerate() {
            let icon = if index == cursor { "●" } else { "○" };
            write_line(
                &mut stdout,
                &format!("{} {} ({})", icon, label, description),
            )?;
        }

        write_line(&mut stdout, "")?;
        write_line(&mut stdout, "Use Up/Down to move, Enter to confirm.")?;
        stdout.flush()?;

        match read_key()? {
            Key::Up => {
                cursor = cursor.saturating_sub(1);
            }
            Key::Down => {
                if cursor + 1 < scopes.len() {
                    cursor += 1;
                }
            }
            Key::Confirm => return Ok(scopes[cursor].0),
            Key::Cancel => return Err(Error::Skills("Installation cancelled.".into())),
            Key::Toggle => {}
        }
    }
}

enum Key {
    Up,
    Down,
    Toggle,
    Confirm,
    Cancel,
}

fn read_key() -> Result<Key> {
    let mut stdin = io::stdin();
    let mut buf = [0; 1];

    loop {
        stdin.read_exact(&mut buf)?;

        match buf[0] {
            b' ' => return Ok(Key::Toggle),
            b'\r' | b'\n' => return Ok(Key::Confirm),
            b'k' => return Ok(Key::Up),
            b'j' => return Ok(Key::Down),
            b'q' | 3 => return Ok(Key::Cancel),
            27 => {
                let mut seq = [0; 2];
                stdin.read_exact(&mut seq)?;
                match seq {
                    [b'[', b'A'] => return Ok(Key::Up),
                    [b'[', b'B'] => return Ok(Key::Down),
                    _ => return Ok(Key::Cancel),
                }
            }
            _ => continue,
        }
    }
}

struct TerminalUi {
    fd: i32,
    original: libc::termios,
}

impl TerminalUi {
    fn new(stdout: &mut io::Stdout) -> Result<Self> {
        let fd = libc::STDIN_FILENO;
        let mut original = libc::termios {
            c_iflag: 0,
            c_oflag: 0,
            c_cflag: 0,
            c_lflag: 0,
            c_cc: [0; libc::NCCS],
            c_ispeed: 0,
            c_ospeed: 0,
        };

        let get_attr_result = unsafe { libc::tcgetattr(fd, &mut original) };
        if get_attr_result != 0 {
            return Err(io::Error::last_os_error().into());
        }

        let mut raw = original;
        raw.c_iflag &= !(libc::BRKINT | libc::ICRNL | libc::INPCK | libc::ISTRIP | libc::IXON);
        raw.c_oflag &= !libc::OPOST;
        raw.c_cflag |= libc::CS8;
        raw.c_lflag &= !(libc::ECHO | libc::ICANON | libc::IEXTEN | libc::ISIG);
        raw.c_cc[libc::VMIN] = 1;
        raw.c_cc[libc::VTIME] = 0;

        let set_attr_result = unsafe { libc::tcsetattr(fd, libc::TCSAFLUSH, &raw) };
        if set_attr_result != 0 {
            return Err(io::Error::last_os_error().into());
        }

        write!(stdout, "\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H")?;
        stdout.flush()?;

        Ok(Self { fd, original })
    }
}

impl Drop for TerminalUi {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSAFLUSH, &self.original);
        }
        let mut stdout = io::stdout();
        let _ = write!(stdout, "\x1b[?25h\x1b[?1049l");
        let _ = stdout.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tar::Builder;

    #[test]
    fn detects_agents_from_home_dirs() {
        let home = temp_test_dir("detect");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        std::fs::create_dir_all(home.join(".codex")).unwrap();
        std::fs::create_dir_all(home.join(".agents")).unwrap();

        let detected = detect_agents(&home);
        let keys = detected.iter().map(|agent| agent.key).collect::<Vec<_>>();

        assert!(keys.contains(&"claude"));
        assert!(keys.contains(&"agents"));
        assert!(keys.contains(&"codex"));
        assert!(!keys.contains(&"cursor"));

        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn extracts_only_skill_files_from_repo_archive() {
        let archive = build_test_archive(&[
            (
                "agent-skills-main/skills/clickhouse-best-practices/SKILL.md",
                b"best practices".as_slice(),
            ),
            (
                "agent-skills-main/skills/clickhouse-cli/SKILL.md",
                b"cli skill".as_slice(),
            ),
            ("agent-skills-main/README.md", b"ignore me".as_slice()),
        ]);

        let extracted = extract_skills_from_tarball(&archive.path).unwrap();
        let skill_files = collect_skill_files(&extracted.path).unwrap();
        let paths = skill_files
            .iter()
            .map(|file| {
                format!(
                    "{}/{}",
                    file.skill_slug,
                    file.relative_path.to_string_lossy()
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(
            paths,
            vec![
                "clickhouse-best-practices/SKILL.md",
                "clickhouse-cli/SKILL.md"
            ]
        );
    }

    #[test]
    fn rejects_unsafe_skill_archive_paths() {
        assert!(
            parse_skill_archive_path(Path::new(
                "agent-skills-main/skills/../clickhouse-best-practices/SKILL.md"
            ))
            .is_none()
        );
        assert!(
            parse_skill_archive_path(Path::new(
                "agent-skills-main/skills/clickhouse-safe/../../SKILL.md"
            ))
            .is_none()
        );
        assert!(
            parse_skill_archive_path(Path::new(
                "agent-skills-main/skills/clickhouse-safe/SKILL.md"
            ))
            .is_some()
        );
    }

    #[test]
    fn installs_downloaded_skill_files_into_agent_directory() {
        let root = temp_test_dir("install");
        let staging = temp_test_dir("staging");
        std::fs::create_dir_all(staging.join("clickhouse-best-practices")).unwrap();
        std::fs::create_dir_all(staging.join("clickhouse-cli/examples")).unwrap();
        std::fs::write(
            staging.join("clickhouse-best-practices/SKILL.md"),
            b"best practices",
        )
        .unwrap();
        std::fs::write(
            staging.join("clickhouse-cli/examples/query.sql"),
            b"SELECT 1",
        )
        .unwrap();
        let skill_files = vec![
            SkillFile {
                skill_slug: "clickhouse-best-practices".into(),
                relative_path: PathBuf::from("SKILL.md"),
                source_path: staging.join("clickhouse-best-practices/SKILL.md"),
            },
            SkillFile {
                skill_slug: "clickhouse-cli".into(),
                relative_path: PathBuf::from("examples/query.sql"),
                source_path: staging.join("clickhouse-cli/examples/query.sql"),
            },
        ];

        let agent = find_agent("claude").unwrap();
        let summary = install_into_agent(&root, agent, &skill_files).unwrap();
        assert_eq!(summary.created_files, 2);
        assert!(
            root.join(".claude/skills/clickhouse-best-practices/SKILL.md")
                .is_file()
        );
        assert!(
            root.join(".claude/skills/clickhouse-cli/examples/query.sql")
                .is_file()
        );

        let second_summary = install_into_agent(&root, agent, &skill_files).unwrap();
        assert_eq!(second_summary.unchanged_files, 2);

        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(staging).unwrap();
    }

    #[test]
    fn orders_detected_agents_before_supported_undetected_agents() {
        let detected = vec![
            universal_agent(),
            find_agent("codex").unwrap(),
            find_agent("claude").unwrap(),
        ];
        let ordered = ordered_additional_agents(&detected);
        let keys = ordered.iter().map(|agent| agent.key).collect::<Vec<_>>();

        assert_eq!(keys[0], "codex");
        assert_eq!(keys[1], "claude");
        assert!(keys.contains(&"cursor"));
        assert!(keys.contains(&"windsurf"));
        assert!(!keys.contains(&"agents"));
    }

    #[test]
    fn all_selects_every_supported_agent() {
        let install_root = temp_test_dir("all-install");
        let home = temp_test_dir("all-home");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        let detected = detect_agents(&home);
        let args = SkillsArgs {
            agents: Vec::new(),
            all: true,
            detected_only: false,
            global: false,
        };

        let selected = resolve_selection(&install_root, &detected, &args).unwrap();
        assert_eq!(selected.len(), SUPPORTED_AGENTS.len());

        std::fs::remove_dir_all(install_root).unwrap();
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn detected_only_selects_universal_plus_detected_agents() {
        let install_root = temp_test_dir("detected-install");
        let home = temp_test_dir("detected-home");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        std::fs::create_dir_all(home.join(".windsurf")).unwrap();
        let detected = detect_agents(&home);
        let args = SkillsArgs {
            agents: Vec::new(),
            all: false,
            detected_only: true,
            global: false,
        };

        let selected = resolve_selection(&install_root, &detected, &args).unwrap();
        let keys = selected.iter().map(|agent| agent.key).collect::<Vec<_>>();

        assert_eq!(keys, vec!["agents", "claude", "windsurf"]);

        std::fs::remove_dir_all(install_root).unwrap();
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn scope_root_uses_current_dir_for_project() {
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(scope_root(InstallScope::Project).unwrap(), cwd);
    }

    fn build_test_archive(entries: &[(&str, &[u8])]) -> TempArtifact {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = Builder::new(encoder);

        for (path, contents) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_path(path).unwrap();
            header.set_size(contents.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append(&header, *contents).unwrap();
        }

        let encoder = builder.into_inner().unwrap();
        let archive_bytes = encoder.finish().unwrap();
        let artifact = create_temp_artifact("clickhousectl-skills-test", ".tar.gz", false).unwrap();
        std::fs::write(&artifact.path, archive_bytes).unwrap();
        artifact
    }

    fn temp_test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("clickhousectl-skills-{name}-{unique}"));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
