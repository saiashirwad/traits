//! File discovery, trait-ignore directives, and path resolution.
//!
//! Ports `getIncludedFiles.ts` and `traitIgnores.ts`. The Node implementation
//! resolves include patterns with `FileSystem.glob` (picomatch under the hood);
//! the Rust implementation walks from the longest non-glob prefix of each
//! pattern and matches with `globset`. Documented deviations from `fs.glob`:
//! - Hidden entries are skipped unless the supplied pattern names a dot
//!   component; `fs.glob` applies its own per-segment dot rules.
//! - A configured exclusion without a path separator matches the basename at
//!   any depth; `fs.glob` exclude semantics for bare names are ambiguous.
//! - `.git` and `.worktrees` directories are always skipped, independent of the
//!   pattern, so an explicit pattern cannot opt back into them.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

use globset::{GlobBuilder, GlobMatcher};
use ignore::WalkBuilder;
use regex::Regex;

use crate::config::Config;
use crate::doc::Trait;
use crate::error::ToolFailure;

static IGNORE_DIRECTIVE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^//\s*traits-ignore-file\s+(\S+)\s*:\s*(\S.*)$").expect("valid directive regex"));

/// Resolve a path against `root`, normalizing `.` and `..` lexically. Matches
/// the lexical behavior of `path.resolve` (no symlink resolution).
pub(crate) fn absolute(root: &Path, path: &Path) -> PathBuf {
    let joined = if path.is_absolute() { path.to_path_buf() } else { root.join(path) };
    normalize(&joined)
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The set of files Git tracks or leaves untracked but not ignored, resolved
/// against the config root. `Ok(None)` when the Git filter is disabled.
pub async fn git_included_files(config: &Config) -> Result<Option<HashSet<PathBuf>>, ToolFailure> {
    if !config.exclude_gitignored {
        return Ok(None);
    }
    let root = &config.config_root;
    let output = tokio::process::Command::new("git")
        .args(["ls-files", "--cached", "--others", "--exclude-standard", "--full-name", "-z"])
        .current_dir(root)
        .output()
        .await
        .map_err(|error| ToolFailure::platform(format!("failed to run git ls-files: {error}")))?;
    if !output.status.success() {
        let exit_code = output.status.code().unwrap_or(1);
        return Err(ToolFailure::git_listing(&root.to_string_lossy(), exit_code));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut files = HashSet::new();
    for entry in text.split('\0') {
        if entry.is_empty() {
            continue;
        }
        files.insert(absolute(root, Path::new(entry)));
    }
    Ok(Some(files))
}

/// Expand include patterns into absolute regular-file paths, preserving
/// first-seen order and deduplicating exact paths.
pub fn resolve_includes(config: &Config, includes: &[String]) -> Result<Vec<PathBuf>, ToolFailure> {
    let root = &config.config_root;
    let excludes = ExcludeSet::build(config)?;
    let mut seen = HashSet::new();
    let mut resolved = Vec::new();

    for pattern in includes {
        let absolute_pattern = absolute(root, Path::new(pattern));
        let pattern_text = absolute_pattern.to_string_lossy();
        if !has_glob_meta(&pattern_text) {
            if let Ok(metadata) = std::fs::metadata(&absolute_pattern)
                && metadata.is_file()
                && !has_always_excluded_component(&absolute_pattern)
                && !excludes.matches(&absolute_pattern)
                && seen.insert(absolute_pattern.clone())
            {
                resolved.push(absolute_pattern);
            }
            continue;
        }

        let glob = GlobBuilder::new(&pattern_text)
            .literal_separator(true)
            .build()
            .map_err(|error| ToolFailure::platform(format!("invalid include pattern {pattern:?}: {error}")))?
            .compile_matcher();
        let allowed_hidden = names_dot_component(pattern);
        let base = glob_base(&absolute_pattern);
        if !base.is_dir() {
            continue;
        }

        let mut builder = WalkBuilder::new(&base);
        builder
            .hidden(false)
            .parents(false)
            .ignore(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .follow_links(true);
        builder.filter_entry(move |entry| {
            if entry.depth() == 0 {
                return true;
            }
            let name = entry.file_name().to_string_lossy();
            if name == ".git" || name == ".worktrees" {
                return false;
            }
            allowed_hidden || !name.starts_with('.')
        });

        for entry in builder.build() {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            if !glob.is_match(path) {
                continue;
            }
            let Ok(metadata) = std::fs::metadata(path) else { continue };
            if !metadata.is_file() {
                continue;
            }
            if !allowed_hidden && has_hidden_component(path, &base) {
                continue;
            }
            if has_always_excluded_component(path) || excludes.matches(path) {
                continue;
            }
            let normalized = normalize(path);
            if seen.insert(normalized.clone()) {
                resolved.push(normalized);
            }
        }
    }

    Ok(resolved)
}

/// The longest non-glob prefix of `pattern`, used as the walk root.
fn glob_base(pattern: &Path) -> PathBuf {
    let mut base = PathBuf::new();
    for component in pattern.components() {
        match component {
            Component::Normal(name) if has_glob_meta(&name.to_string_lossy()) => return base,
            other => base.push(other.as_os_str()),
        }
    }
    base
}

fn has_glob_meta(text: &str) -> bool {
    text.chars().any(|character| matches!(character, '*' | '?' | '[' | ']' | '{' | '}'))
}

fn names_dot_component(pattern: &str) -> bool {
    Path::new(pattern).components().any(|component| match component {
        Component::Normal(name) => {
            let name = name.to_string_lossy();
            name.starts_with('.') && name != "." && name != ".."
        }
        _ => false,
    })
}

fn has_hidden_component(path: &Path, base: &Path) -> bool {
    let relative = path.strip_prefix(base).unwrap_or(path);
    relative.components().any(|component| match component {
        Component::Normal(name) => name.to_string_lossy().starts_with('.'),
        _ => false,
    })
}

fn has_always_excluded_component(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(name) => {
            let name = name.to_string_lossy();
            name == ".git" || name == ".worktrees"
        }
        _ => false,
    })
}

/// Configured exclusions. Patterns with a path separator match the absolute
/// path; bare names match a file's basename at any depth.
struct ExcludeSet {
    path_globs: Vec<GlobMatcher>,
    name_globs: Vec<GlobMatcher>,
}

impl ExcludeSet {
    fn build(config: &Config) -> Result<Self, ToolFailure> {
        let mut path_globs = Vec::new();
        let mut name_globs = Vec::new();
        for pattern in &config.excludes {
            if pattern.contains('/') || pattern.contains('\\') {
                let absolute_pattern = absolute(&config.config_root, Path::new(pattern));
                path_globs.push(build_glob(&absolute_pattern, pattern)?);
            } else {
                name_globs.push(build_glob(Path::new(pattern), pattern)?);
            }
        }
        Ok(Self { path_globs, name_globs })
    }

    fn matches(&self, path: &Path) -> bool {
        if self.path_globs.iter().any(|glob| glob.is_match(path)) {
            return true;
        }
        match path.file_name() {
            Some(name) => self.name_globs.iter().any(|glob| glob.is_match(Path::new(name))),
            None => false,
        }
    }
}

fn build_glob(pattern: &Path, source: &str) -> Result<GlobMatcher, ToolFailure> {
    GlobBuilder::new(&pattern.to_string_lossy())
        .literal_separator(true)
        .build()
        .map(|glob| glob.compile_matcher())
        .map_err(|error| ToolFailure::platform(format!("invalid exclude pattern {source:?}: {error}")))
}

/// Parse leading `// traits-ignore-file <reference>: <reason>` directives from a
/// file header. Directives require exactly one reference and a non-empty reason
/// and must appear before any code.
pub fn trait_ignores(text: &str) -> HashSet<String> {
    let mut references = HashSet::new();
    for line in text.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("#!") {
            continue;
        }
        if !trimmed.starts_with("//") {
            break;
        }
        if let Some(captures) = IGNORE_DIRECTIVE.captures(trimmed) {
            references.insert(captures[1].to_string());
        }
    }
    references
}

/// Drop traits whose exact name appears in the file's ignore directives.
pub fn exclude_ignored(traits: &[Trait], text: &str) -> Vec<Trait> {
    let references = trait_ignores(text);
    traits
        .iter()
        .filter(|trait_value| !references.contains(trait_value.name.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::TraitName;

    fn trait_named(name: &str) -> Trait {
        Trait {
            name: TraitName(name.to_string()),
            subject: "Subject.".to_string(),
            requirement: "Requirement.".to_string(),
            applicability: None,
            purpose: "Purpose.".to_string(),
            guidance: None,
            context: None,
        }
    }

    fn test_config(root: &Path, excludes: Vec<String>) -> Config {
        Config {
            config_root: root.to_path_buf(),
            store: root.join("traits"),
            concurrency: 4,
            exclude_gitignored: false,
            excludes,
            default_provider: None,
            classify: Default::default(),
            generate: Default::default(),
            providers: Vec::new(),
        }
    }

    #[test]
    fn reads_repeated_directives_from_header() {
        let references = trait_ignores(
            "#!/usr/bin/env node\n\n// header\n// traits-ignore-file first: reason\n// traits-ignore-file second: another reason\nimport 'module'",
        );
        assert_eq!(references, HashSet::from(["first".to_string(), "second".to_string()]));
    }

    #[test]
    fn requires_a_reason_and_exactly_one_reference() {
        let references = trait_ignores(
            "// traits-ignore-file first\n// traits-ignore-file second:   \n// traits-ignore-file first second: reason",
        );
        assert!(references.is_empty());
    }

    #[test]
    fn ignores_strings_and_comments_after_code() {
        let references =
            trait_ignores("const text = `\n// traits-ignore-file first: reason\n`\n// traits-ignore-file second: reason");
        assert!(references.is_empty());
    }

    #[test]
    fn handles_crlf_and_deduplicates() {
        let references = trait_ignores("// traits-ignore-file first: reason\r\n// traits-ignore-file first: repeated\r\n");
        assert_eq!(references, HashSet::from(["first".to_string()]));
    }

    #[test]
    fn selects_exact_unique_names_and_multiple_exceptions() {
        let first = trait_named("direct-callbacks");
        let second = trait_named("use-arrow-functions");
        let selected = exclude_ignored(&[first.clone(), second.clone()], "// traits-ignore-file direct-callbacks: exception");
        assert_eq!(selected, vec![second.clone()]);
        let header = "// traits-ignore-file direct-callbacks: first exception\n// traits-ignore-file use-arrow-functions: second exception\nconst value = 1";
        assert!(exclude_ignored(&[first, second], header).is_empty());
    }

    #[test]
    fn rejects_partial_casing_filename_and_unknown_references() {
        let first = trait_named("direct-callbacks");
        let second = trait_named("use-arrow-functions");
        let header = "// traits-ignore-file direct: partial\n// traits-ignore-file Direct-callbacks: wrong casing\n// traits-ignore-file direct-callbacks.md: filename\n// traits-ignore-file unknown-trait: unknown";
        assert_eq!(exclude_ignored(&[first.clone(), second.clone()], header), vec![first, second]);
    }

    #[test]
    fn requires_references_to_be_updated_after_renaming() {
        let renamed = trait_named("renamed-callbacks");
        assert_eq!(
            exclude_ignored(std::slice::from_ref(&renamed), "// traits-ignore-file direct-callbacks: old name"),
            vec![renamed.clone()]
        );
        assert!(
            exclude_ignored(std::slice::from_ref(&renamed), "// traits-ignore-file renamed-callbacks: new name")
                .is_empty()
        );
    }

    #[test]
    fn ignores_directives_without_reasons_or_after_code() {
        let first = trait_named("direct-callbacks");
        assert_eq!(
            exclude_ignored(std::slice::from_ref(&first), "// traits-ignore-file direct-callbacks:   "),
            vec![first.clone()]
        );
        assert_eq!(
            exclude_ignored(
                std::slice::from_ref(&first),
                "const value = 1\n// traits-ignore-file direct-callbacks: too late"
            ),
            vec![first]
        );
    }

    #[test]
    fn resolves_relative_and_absolute_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src/nested")).unwrap();
        std::fs::write(root.join("src/a.ts"), "a").unwrap();
        std::fs::write(root.join("src/nested/b.ts"), "b").unwrap();
        let config = test_config(root, Vec::new());

        let relative = resolve_includes(&config, &["src/**/*.ts".to_string()]).unwrap();
        assert_eq!(
            relative,
            vec![root.join("src/a.ts"), root.join("src/nested/b.ts")],
            "relative glob resolves against the config root"
        );

        let absolute_pattern = root.join("src/*.ts").to_string_lossy().into_owned();
        let absolute = resolve_includes(&config, &[absolute_pattern]).unwrap();
        assert_eq!(absolute, vec![root.join("src/a.ts")]);
    }

    #[test]
    fn handles_dotfiles_and_unmatched_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/visible.ts"), "v").unwrap();
        std::fs::write(root.join("src/.hidden.ts"), "h").unwrap();
        let config = test_config(root, Vec::new());

        let default = resolve_includes(&config, &["src/*.ts".to_string()]).unwrap();
        assert_eq!(default, vec![root.join("src/visible.ts")]);

        let explicit = resolve_includes(&config, &["src/.hidden.ts".to_string()]).unwrap();
        assert_eq!(explicit, vec![root.join("src/.hidden.ts")]);

        assert!(resolve_includes(&config, &["does/not/exist/*.ts".to_string()]).unwrap().is_empty());
    }

    #[test]
    fn applies_excludes_and_deduplicates() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/keep.ts"), "k").unwrap();
        std::fs::write(root.join("src/skip.ts"), "s").unwrap();
        std::fs::write(root.join("src/other.ts"), "o").unwrap();
        let config = test_config(root, vec!["src/skip.ts".to_string()]);

        let resolved = resolve_includes(
            &config,
            &[
                "src/other.ts".to_string(),
                "src/keep.ts".to_string(),
                "src/other.ts".to_string(),
                "src/*.ts".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(resolved, vec![root.join("src/other.ts"), root.join("src/keep.ts")]);
    }

    #[test]
    fn skips_git_and_worktrees_directories() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".git/objects")).unwrap();
        std::fs::create_dir_all(root.join(".worktrees/copy")).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join(".git/objects/ignored.ts"), "x").unwrap();
        std::fs::write(root.join(".worktrees/copy/ignored.ts"), "x").unwrap();
        std::fs::write(root.join("src/kept.ts"), "k").unwrap();
        let config = test_config(root, Vec::new());

        let resolved = resolve_includes(&config, &["**/*.ts".to_string()]).unwrap();
        assert_eq!(resolved, vec![root.join("src/kept.ts")]);
    }

    #[tokio::test]
    async fn git_included_files_returns_none_when_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = test_config(dir.path(), Vec::new());
        config.exclude_gitignored = false;
        assert!(git_included_files(&config).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn git_included_files_lists_tracked_and_untracked() {
        if std::process::Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("tracked.ts"), "t").unwrap();
        std::fs::write(root.join("untracked.ts"), "u").unwrap();
        std::fs::write(root.join(".gitignore"), "ignored.ts\n").unwrap();
        std::fs::write(root.join("ignored.ts"), "i").unwrap();
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap();
            assert!(status.success(), "git {args:?} failed");
        };
        run(&["init", "-q"]);
        run(&["add", "tracked.ts"]);
        let mut config = test_config(root, Vec::new());
        config.exclude_gitignored = true;
        let files = git_included_files(&config).await.unwrap().unwrap();
        assert!(files.contains(&root.join("tracked.ts")));
        assert!(files.contains(&root.join("untracked.ts")));
        assert!(!files.contains(&root.join("ignored.ts")));
    }
}
