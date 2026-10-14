//! `nemonik example`: lists the example kits under `examples/` on the repo's `main` and
//! writes one into a local folder.

use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

pub const REPO: &str = "mitchldtn/nemonik";
const REF: &str = "main";

#[derive(Debug, PartialEq, Eq)]
pub struct ExampleFile {
    pub path: PathBuf,
    pub contents: Vec<u8>,
}

/// Every example in the archive, by name, with its files at paths relative to the example.
pub type Examples = BTreeMap<String, Vec<ExampleFile>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetState {
    Missing,
    EmptyDir,
    NonEmptyDir,
    NotADir,
}

fn tarball_url() -> String {
    format!("https://api.github.com/repos/{REPO}/tarball/{REF}")
}

/// Reads a gzipped repo tarball. Only regular files under `<top>/examples/<name>/` are
/// kept; links are skipped, and a `..` or absolute path there fails the whole archive.
pub fn parse_archive(gz: &[u8]) -> Result<Examples> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(gz));
    let mut examples = Examples::new();
    let entries = archive
        .entries()
        .context("the examples download is not a valid archive")?;
    for entry in entries {
        let mut entry = entry.context("the examples download is not a valid archive")?;
        let path = entry
            .path()
            .context("the examples archive has an unreadable path")?
            .into_owned();
        if !under_examples(&path) {
            continue;
        }
        let parts = safe_components(&path)?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let [_top, _examples, name, rest @ ..] = parts.as_slice() else {
            continue;
        };
        if rest.is_empty() {
            continue;
        }
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents).with_context(|| {
            format!(
                "reading {} from the examples archive failed",
                path.display()
            )
        })?;
        examples.entry(name.clone()).or_default().push(ExampleFile {
            path: rest.iter().collect(),
            contents,
        });
    }
    Ok(examples)
}

fn under_examples(path: &Path) -> bool {
    let mut components = path.components().filter(|c| *c != Component::CurDir);
    components.next();
    components.next() == Some(Component::Normal("examples".as_ref()))
}

fn safe_components(path: &Path) -> Result<Vec<String>> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => match part.to_str() {
                Some(part) => parts.push(part.to_string()),
                None => bail!("unsafe path in the examples archive: {}", path.display()),
            },
            Component::CurDir => {}
            _ => bail!("unsafe path in the examples archive: {}", path.display()),
        }
    }
    Ok(parts)
}

/// The first non-blank line of a README, without leading `#` marks.
pub fn readme_summary(readme: &str) -> String {
    readme
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.trim_start_matches('#').trim().to_string())
        .unwrap_or_default()
}

pub fn summaries(examples: &Examples) -> Vec<(String, String)> {
    examples
        .iter()
        .map(|(name, files)| {
            let summary = files
                .iter()
                .find(|f| f.path == Path::new("README.md"))
                .map(|f| readme_summary(&String::from_utf8_lossy(&f.contents)))
                .unwrap_or_default();
            (name.clone(), summary)
        })
        .collect()
}

pub fn select(mut examples: Examples, name: &str) -> Result<Vec<ExampleFile>> {
    if let Some(files) = examples.remove(name) {
        return Ok(files);
    }
    let available: Vec<&str> = examples.keys().map(String::as_str).collect();
    if available.is_empty() {
        bail!("no example named '{name}': no examples are available");
    }
    bail!(
        "no example named '{name}'. Available: {}",
        available.join(", ")
    )
}

fn check_target(dir: &Path, state: TargetState) -> Result<()> {
    match state {
        TargetState::Missing | TargetState::EmptyDir => Ok(()),
        TargetState::NonEmptyDir => bail!(
            "{} already exists and is not empty; pass another folder as the second argument",
            dir.display()
        ),
        TargetState::NotADir => bail!("{} already exists and is not a folder", dir.display()),
    }
}

/// Trimmed; blank counts as unset.
pub fn token_from_env(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

pub fn status_error(status: u16) -> String {
    match status {
        403 | 429 => format!(
            "GitHub rate-limited the examples download (HTTP {status}). Set GITHUB_TOKEN to \
             raise the limit."
        ),
        _ => format!("downloading the examples failed (HTTP {status})."),
    }
}

fn target_state(dir: &Path) -> Result<TargetState> {
    match std::fs::metadata(dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(TargetState::Missing),
        Err(e) => Err(e).with_context(|| format!("failed to inspect {}", dir.display())),
        Ok(meta) if !meta.is_dir() => Ok(TargetState::NotADir),
        Ok(_) => {
            let mut entries = std::fs::read_dir(dir)
                .with_context(|| format!("failed to read {}", dir.display()))?;
            Ok(if entries.next().is_none() {
                TargetState::EmptyDir
            } else {
                TargetState::NonEmptyDir
            })
        }
    }
}

/// Writes the example into `dir`, which must be missing or empty. Never overwrites.
pub fn install_example(dir: &Path, files: &[ExampleFile]) -> Result<()> {
    check_target(dir, target_state(dir)?)?;
    for file in files {
        let path = dir.join(&file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .and_then(|mut f| f.write_all(&file.contents))
            .with_context(|| format!("failed to write {}", path.display()))?;
    }
    Ok(())
}

/// Downloads the repo tarball. reqwest drops `Authorization` when GitHub redirects to
/// codeload, so the token only reaches api.github.com.
pub async fn fetch_tarball(token: Option<&str>) -> Result<Vec<u8>> {
    // reqwest has no default timeout; without these a stalled download hangs.
    let client = reqwest::Client::builder()
        .user_agent(crate::mcp::USER_AGENT)
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("building the HTTP client for the examples download failed")?;
    let mut request = client.get(tarball_url());
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let response = request
        .send()
        .await
        .context("downloading the examples from GitHub failed")?;
    let status = response.status();
    if !status.is_success() {
        bail!(status_error(status.as_u16()));
    }
    let body = response
        .bytes()
        .await
        .context("downloading the examples from GitHub failed")?;
    Ok(body.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    enum Entry<'a> {
        File(&'a str, &'a str),
        Dir(&'a str),
        Symlink(&'a str, &'a str),
        Hardlink(&'a str, &'a str),
    }

    // Writes the name straight into the header so tests can build paths `tar::Builder`
    // refuses, such as ones containing `..`.
    fn header(name: &str, kind: tar::EntryType, size: u64) -> tar::Header {
        let mut h = tar::Header::new_old();
        h.as_old_mut().name[..name.len()].copy_from_slice(name.as_bytes());
        h.set_entry_type(kind);
        h.set_size(size);
        h.set_mode(0o644);
        h
    }

    fn archive(entries: &[Entry]) -> Vec<u8> {
        let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut builder = tar::Builder::new(gz);
        for entry in entries {
            match entry {
                Entry::File(path, body) => {
                    let mut h = header(path, tar::EntryType::Regular, body.len() as u64);
                    h.set_cksum();
                    builder.append(&h, body.as_bytes()).unwrap();
                }
                Entry::Dir(path) => {
                    let mut h = header(path, tar::EntryType::Directory, 0);
                    h.set_cksum();
                    builder.append(&h, &[][..]).unwrap();
                }
                Entry::Symlink(path, target) | Entry::Hardlink(path, target) => {
                    let kind = if matches!(entry, Entry::Symlink(..)) {
                        tar::EntryType::Symlink
                    } else {
                        tar::EntryType::Link
                    };
                    let mut h = header(path, kind, 0);
                    h.as_old_mut().linkname[..target.len()].copy_from_slice(target.as_bytes());
                    h.set_cksum();
                    builder.append(&h, &[][..]).unwrap();
                }
            }
        }
        let mut gz = builder.into_inner().unwrap();
        gz.flush().unwrap();
        gz.finish().unwrap()
    }

    fn repo_archive() -> Vec<u8> {
        archive(&[
            Entry::Dir("o-r-abc/"),
            Entry::File("o-r-abc/README.md", "# the repo\n"),
            Entry::File("o-r-abc/src/main.rs", "fn main() {}\n"),
            Entry::Dir("o-r-abc/examples/"),
            Entry::Dir("o-r-abc/examples/local-kit/"),
            Entry::File(
                "o-r-abc/examples/local-kit/README.md",
                "\n# One kit, every part\n\nBody.\n",
            ),
            Entry::File("o-r-abc/examples/local-kit/nemonik.json", "{}\n"),
            Entry::File("o-r-abc/examples/local-kit/skills/tour/SKILL.md", "skill\n"),
            Entry::File("o-r-abc/examples/pair/README.md", "Two agents\n"),
            Entry::File("o-r-abc/examples/pair/kit/coder.md", "coder\n"),
            Entry::File("o-r-abc/examples/no-readme/nemonik.json", "{}\n"),
        ])
    }

    fn paths(files: &[ExampleFile]) -> Vec<String> {
        files.iter().map(|f| f.path.display().to_string()).collect()
    }

    #[test]
    fn parse_archive_groups_files_by_example_and_strips_the_prefix() {
        let examples = parse_archive(&repo_archive()).unwrap();
        assert_eq!(
            examples.keys().collect::<Vec<_>>(),
            ["local-kit", "no-readme", "pair"]
        );
        let mut files = paths(&examples["local-kit"]);
        files.sort();
        assert_eq!(files, ["README.md", "nemonik.json", "skills/tour/SKILL.md"]);
        let skill = examples["local-kit"]
            .iter()
            .find(|f| f.path.ends_with("SKILL.md"))
            .unwrap();
        assert_eq!(skill.contents, b"skill\n");
    }

    #[test]
    fn parse_archive_skips_files_directly_under_examples() {
        let gz = archive(&[
            Entry::File("o-r-abc/examples/README.md", "index\n"),
            Entry::File("o-r-abc/examples/x/README.md", "x\n"),
        ]);
        let examples = parse_archive(&gz).unwrap();
        assert_eq!(examples.keys().collect::<Vec<_>>(), ["x"]);
    }

    #[test]
    fn parse_archive_ignores_files_outside_examples() {
        let examples = parse_archive(&repo_archive()).unwrap();
        let all: Vec<String> = examples.values().flat_map(|f| paths(f)).collect();
        assert!(!all.iter().any(|p| p.contains("main.rs")), "{all:?}");
    }

    #[test]
    fn parse_archive_rejects_a_parent_dir_component() {
        let gz = archive(&[Entry::File("o-r-abc/examples/x/../../../evil", "boom")]);
        let err = parse_archive(&gz).unwrap_err().to_string();
        assert!(err.contains("unsafe path"), "{err}");
    }

    #[test]
    fn parse_archive_rejects_an_absolute_path() {
        let gz = archive(&[Entry::File("/examples/x/evil", "boom")]);
        let err = parse_archive(&gz).unwrap_err().to_string();
        assert!(err.contains("unsafe path"), "{err}");
    }

    // Entries outside `examples/` are never written, so an odd path there must not
    // break the command.
    #[test]
    fn parse_archive_ignores_odd_paths_outside_examples() {
        let gz = archive(&[
            Entry::File("/etc/evil", "boom"),
            Entry::File("o-r-abc/../outside", "boom"),
            Entry::File("o-r-abc/examples/x/README.md", "x\n"),
        ]);
        let examples = parse_archive(&gz).unwrap();
        assert_eq!(paths(&examples["x"]), ["README.md"]);
    }

    #[test]
    fn parse_archive_skips_symlinks_and_hardlinks() {
        let gz = archive(&[
            Entry::File("o-r-abc/examples/x/README.md", "x\n"),
            Entry::Symlink("o-r-abc/examples/x/passwd", "/etc/passwd"),
            Entry::Hardlink("o-r-abc/examples/x/hard", "o-r-abc/README.md"),
        ]);
        let examples = parse_archive(&gz).unwrap();
        assert_eq!(paths(&examples["x"]), ["README.md"]);
    }

    #[test]
    fn readme_summary_is_the_first_nonblank_line_without_heading_marks() {
        assert_eq!(readme_summary("\n\n## Two agents  \nmore"), "Two agents");
        assert_eq!(readme_summary("Plain first line\n"), "Plain first line");
        assert_eq!(readme_summary(""), "");
    }

    #[test]
    fn summaries_lists_every_example_with_its_readme_line() {
        let examples = parse_archive(&repo_archive()).unwrap();
        assert_eq!(
            summaries(&examples),
            [
                ("local-kit".to_string(), "One kit, every part".to_string()),
                ("no-readme".to_string(), String::new()),
                ("pair".to_string(), "Two agents".to_string()),
            ]
        );
    }

    #[test]
    fn select_returns_the_named_examples_files() {
        let examples = parse_archive(&repo_archive()).unwrap();
        let mut files = paths(&select(examples, "pair").unwrap());
        files.sort();
        assert_eq!(files, ["README.md", "kit/coder.md"]);
    }

    #[test]
    fn select_an_unknown_name_lists_the_available_ones() {
        let examples = parse_archive(&repo_archive()).unwrap();
        let err = select(examples, "locl-kit").unwrap_err().to_string();
        assert!(err.contains("locl-kit"), "{err}");
        assert!(err.contains("local-kit, no-readme, pair"), "{err}");
    }

    #[test]
    fn token_from_env_treats_blank_as_unset() {
        assert_eq!(token_from_env(None), None);
        assert_eq!(token_from_env(Some("  ".into())), None);
        assert_eq!(token_from_env(Some("ghp_x".into())), Some("ghp_x".into()));
        assert_eq!(token_from_env(Some("ghp_x\n".into())), Some("ghp_x".into()));
    }

    #[test]
    fn status_error_names_the_rate_limit_only_for_403_and_429() {
        for status in [403, 429] {
            let msg = status_error(status);
            assert!(msg.contains("rate-limited"), "{msg}");
            assert!(msg.contains("Set GITHUB_TOKEN"), "{msg}");
        }
        let msg = status_error(404);
        assert!(msg.contains("HTTP 404"), "{msg}");
        assert!(!msg.contains("GITHUB_TOKEN"), "{msg}");
    }

    fn file(path: &str, body: &str) -> ExampleFile {
        ExampleFile {
            path: path.into(),
            contents: body.as_bytes().to_vec(),
        }
    }

    fn entries(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn install_example_fills_a_missing_or_empty_dir() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("empty")).unwrap();
        for target in ["missing", "empty"] {
            let dir = root.path().join(target);
            install_example(&dir, &[file("a/b.md", "b\n")]).unwrap();
            assert_eq!(std::fs::read(dir.join("a/b.md")).unwrap(), b"b\n");
        }
    }

    #[test]
    fn install_example_refuses_a_non_empty_target() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("demo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mine.md"), "mine").unwrap();
        assert!(install_example(&dir, &[file("README.md", "x")]).is_err());
        assert_eq!(entries(&dir), ["mine.md"]);
    }

    #[test]
    fn install_example_refuses_a_file_target() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("demo");
        std::fs::write(&target, "mine").unwrap();
        let err = install_example(&target, &[file("README.md", "x")])
            .unwrap_err()
            .to_string();
        assert!(err.contains("not a folder"), "{err}");
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "mine");
    }

    fn repo_examples_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples")
    }

    fn files_under(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut out = BTreeMap::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    let rel = path.strip_prefix(root).unwrap().to_path_buf();
                    out.insert(rel, std::fs::read(&path).unwrap());
                }
            }
        }
        out
    }

    // The repo's own examples, packed the way GitHub serves them, come out byte-identical.
    #[test]
    fn repo_examples_round_trip_through_the_archive() {
        let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut builder = tar::Builder::new(gz);
        builder
            .append_dir_all("mitchldtn-nemonik-abc123/examples", repo_examples_dir())
            .unwrap();
        let gz = builder.into_inner().unwrap().finish().unwrap();
        let examples = parse_archive(&gz).unwrap();

        let names: Vec<&str> = examples.keys().map(String::as_str).collect();
        for (name, summary) in summaries(&examples) {
            assert!(!summary.is_empty(), "{name} has no README summary");
        }

        for name in names.iter().map(|n| n.to_string()).collect::<Vec<_>>() {
            let root = tempfile::tempdir().unwrap();
            let dir = root.path().join(&name);
            install_example(&dir, &select(parse_archive(&gz).unwrap(), &name).unwrap()).unwrap();
            assert_eq!(
                files_under(&dir),
                files_under(&repo_examples_dir().join(&name)),
                "{name}"
            );
        }
    }
}
