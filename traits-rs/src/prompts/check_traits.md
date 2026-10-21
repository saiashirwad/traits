 Use after generating or editing TypeScript files, after saving or resolving a relevant trait, and before committing or opening a PR.
 Assess only project-owned source files and relevant authored configuration or documentation. Do not audit third-party or generated code.
 For a whole-project review, prefer an explicit project-file manifest rather than a filesystem-wide glob.
 Never submit secrets, .env files, credentials, binary assets, generated lockfiles, dependency trees, or reference repositories for assessment.
 Supply include as TypeScript file paths or glob patterns relevant to the work, relative to the MCP working directory or absolute.
 Globs are expanded and overlapping matches are checked once. Directories are not reviewed as files; use patterns such as traits/**/*.ts.
 Built-in exclusions cover dependency trees, Git metadata, and nested worktrees. Configure project-specific exclusions through TraitsConfig.excludes.
 Configured exclusions are filenames or native filesystem glob patterns, resolved beside traits.ts. Use directory/** to exclude a subtree; bare directories are not expanded.
 Git-ignored files are excluded by default, with ignore rules refreshed on every call. Untracked files do not need to be added to Git to be evaluated. Tracked files remain eligible even if ignore rules match them. Set excludeGitignored: false in traits.ts to disable this filter; explicit excludes still apply. Git failures fail the check.
 All entries follow native glob syntax, including escaping filenames with glob characters. No custom directory expansion or literal-path classification is applied.
 Reads current full file contents, including unstaged edits, and checks each 200-line chunk against the selected traits in parallel.
 Omit traitNames to check all stored traits; supply unique trait names for a focused check. Unknown names fail the call.
 A leading // traits-ignore-file <trait-name>: <reason> comment skips that trait for the entire file, including focused checks. Names are case-sensitive and must be updated after renaming. Directives require a reason and must precede code.
 Returns findings with the trait, source text, absolute path, 1-based chunk startLine, and assessment classification and probabilities.
 Classification distinguishes violation from inconclusive; probabilities are not calibrated guarantees. No exact offending lines are provided.
 Diagnose uncertainty using the full distribution, not the selected label alone; inspect source and trait before changing code.
 Act on the results: inspect every finding and automatically fix confirmed, applicable violations without waiting for a separate request.
 Make minimal behavior-preserving code changes, then rerun this tool on the corrected files and run relevant project checks.
 Repeat for remaining confirmed violations; returning a findings summary alone does not complete the review.
 Do not weaken a trait to make code pass or change correct code for an inapplicable or inconclusive finding.
 Triage each finding before reporting it. Routine summaries should surface confirmed issues, meaningful unresolved uncertainty, errors, and decision blockers.
 Do not distract the user with rejected, irrelevant false positives; retain raw results in tool output for audit and explain them when requested or materially significant.
 Do not hide actionable findings based only on probability/confidence or claim the automated check was clean when raw findings remain.
 Ask for clarification when a fix changes behavior or exceeds authorized scope.
 The tool diagnoses only; the calling agent is responsible for edits, verification, and reporting unresolved blockers.
 Unmatched patterns select no files. An empty result means no failures for this selection, not a repository-wide guarantee.
 Empty include/traitNames lists, no matched files, or an empty store check no pairs.
