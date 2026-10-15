use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};

use crate::cli::CommitArgs;
use crate::error::{OcError, Result};
use crate::i18n::{is_zh, tr};
use crate::opencode::runner::{RunOptions, Runner};
use crate::opencode::{resolve_model, Command};
use crate::ui;

/// `occmt` — commit uncommitted Git changes through an OpenCode agent.
///
/// Repository discovery, dirty detection and multi-repository confirmation are
/// deterministic work done in Rust. The model only inspects the diff, writes
/// the commit message and creates the commit.
pub async fn execute(args: CommitArgs) -> Result<()> {
    ensure_git()?;

    let root = std::env::current_dir()?;
    let repos = discover_repos(&root)?;
    let changed: Vec<PathBuf> = repos.into_iter().filter(|repo| is_dirty(repo)).collect();

    if changed.is_empty() {
        println!("{}", tr("没有需要提交的修改。", "Nothing to commit."));
        return Ok(());
    }

    if changed.len() > 1 {
        println!();
        println!("{}", repos_found_message(changed.len()));
        for repo in &changed {
            println!("  - {}", display_path(&root, repo));
        }
        println!();

        let confirmed = ui::confirm(tr(
            "是否提交以上所有仓库？[y/N] ",
            "Commit all of the above repositories? [y/N] ",
        ))?;
        if !confirmed {
            println!("{}", tr("已取消。", "Cancelled."));
            return Ok(());
        }
    }

    let model = resolve_model(Command::Commit, &args.model)?;
    let runner = Runner::new(args.debug);
    let prompt = build_prompt(&args);

    let mut failures = 0usize;

    for repo in &changed {
        println!();
        println!(
            "{} {}",
            tr("正在处理：", "Processing:"),
            display_path(&root, repo)
        );

        let options = RunOptions {
            model: model.clone(),
            session_id: None,
            directory: Some(repo.clone()),
            prompt: prompt.clone(),
            ephemeral: true,
        };

        match runner.run(&options).await {
            Ok(_) => match last_commit(repo) {
                Some(line) => println!("{} {}", tr("提交结果：", "Commit:"), line),
                None => ui::warn(tr(
                    "未能读取最新提交信息。",
                    "Could not read the latest commit.",
                )),
            },
            Err(OcError::Interrupted) => return Err(OcError::Interrupted),
            Err(err) => {
                failures += 1;
                ui::warn(&format!("{} {}", tr("处理失败：", "failed:"), err));
            }
        }
    }

    if failures > 0 {
        let zh = format!("处理完成，其中 {} 个仓库提交失败。", failures);
        let en = format!("Finished, but {} repositories failed.", failures);
        return Err(OcError::Message(tr(&zh, &en).to_string()));
    }

    println!();
    if changed.len() == 1 {
        println!(
            "{}",
            tr("提交完成。未执行 push。", "Done. No push performed.")
        );
    } else {
        println!("{}", all_done_message(changed.len()));
    }

    Ok(())
}

fn repos_found_message(count: usize) -> String {
    if is_zh() {
        format!("发现 {} 个存在未提交修改的 Git 仓库：", count)
    } else {
        format!("Found {} Git repositories with uncommitted changes:", count)
    }
}

fn all_done_message(count: usize) -> String {
    if is_zh() {
        format!("全部 {} 个仓库处理完成。未执行 push。", count)
    } else {
        format!("All {} repositories processed. No push performed.", count)
    }
}

fn build_prompt(args: &CommitArgs) -> String {
    let language = if args.en {
        "Write the commit message in English."
    } else if args.zh {
        "Write the commit message in Simplified Chinese. Keep the Conventional Commits type in its standard English form, such as feat, fix, refactor, docs, test, or chore."
    } else {
        "Do not impose a commit-message language yourself. Follow the repository instructions, especially AGENTS.md, if they specify one; otherwise infer the appropriate language from the repository context."
    };

    format!(
        r#"Commit the current repository changes.

Requirements:

1. Inspect and obey repository instructions, including AGENTS.md when present.

2. Inspect Git status and the current diff efficiently. Read only additional
   files that are necessary to understand the changes.

3. Create exactly one appropriate commit for the current uncommitted changes.

4. The commit message MUST follow the Conventional Commits specification.

5. {language}

6. Stage tracked and untracked files that clearly belong to the current change.
   Do NOT stage obviously unrelated files, generated files, temporary files,
   credentials, secrets, dependency directories, or build output.

7. Do not modify source files merely to improve the commit.

8. Do not push.

9. Do not amend, rewrite, squash, or otherwise modify existing commits.

10. Do not create an empty commit.

11. Do not ask for confirmation. The user has already authorized this command
    to create the commit.

12. After committing, briefly report the resulting commit hash and subject.

Be efficient. Inspect the existing changes, determine an appropriate
Conventional Commit message, stage the appropriate files, create one commit,
and stop.
"#
    )
}

fn ensure_git() -> Result<()> {
    StdCommand::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|_| ())
        .map_err(|_| {
            OcError::Message(
                tr(
                    "未找到 git，请先安装或检查 PATH。",
                    "git not found. Install it and make sure it is in PATH.",
                )
                .to_string(),
            )
        })
}

fn discover_repos(root: &Path) -> Result<Vec<PathBuf>> {
    // Inside a work tree: only that repository is handled, even when invoked
    // from a subdirectory.
    if is_inside_work_tree(root) {
        if let Some(top) = git_output(root, &["rev-parse", "--show-toplevel"]) {
            return Ok(vec![canonicalize(Path::new(&top))]);
        }
    }

    let mut repos = Vec::new();
    for depth1 in read_dirs(root) {
        if depth1.join(".git").exists() && is_inside_work_tree(&depth1) {
            repos.push(depth1.clone());
        }
        for depth2 in read_dirs(&depth1) {
            if depth2.join(".git").exists() && is_inside_work_tree(&depth2) {
                repos.push(depth2);
            }
        }
    }

    let mut unique: Vec<PathBuf> = Vec::new();
    for repo in repos {
        let repo = canonicalize(&repo);
        if !unique.contains(&repo) {
            unique.push(repo);
        }
    }

    if unique.is_empty() {
        return Err(OcError::Message(
            tr(
                "当前目录及其两级子目录内没有找到 Git 仓库。",
                "No Git repository found in the current directory or within two levels of subdirectories.",
            )
            .to_string(),
        ));
    }

    Ok(unique)
}

fn read_dirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        dirs.push(path);
    }
    dirs
}

fn git_output(dir: &Path, args: &[&str]) -> Option<String> {
    let output = StdCommand::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn is_inside_work_tree(dir: &Path) -> bool {
    git_output(dir, &["rev-parse", "--is-inside-work-tree"]).as_deref() == Some("true")
}

fn is_dirty(repo: &Path) -> bool {
    git_output(repo, &["status", "--porcelain", "--untracked-files=normal"])
        .map(|output| !output.is_empty())
        .unwrap_or(false)
}

fn last_commit(repo: &Path) -> Option<String> {
    git_output(repo, &["log", "-1", "--format=%h %s"]).filter(|line| !line.is_empty())
}

fn canonicalize(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn display_path(root: &Path, repo: &Path) -> String {
    if repo == root {
        ".".to_string()
    } else if let Ok(relative) = repo.strip_prefix(root) {
        format!("./{}", relative.display())
    } else {
        repo.display().to_string()
    }
}
