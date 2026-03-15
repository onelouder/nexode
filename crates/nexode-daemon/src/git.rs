use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;

use crate::session::VerifyConfig;

static VERIFY_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct GitWorktreeOrchestrator {
    repo_root: PathBuf,
    worktree_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeHandle {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub head: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeOutcome {
    pub source_branch: String,
    pub target_branch: String,
    pub verification: VerificationReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VerificationReport {
    pub build: Option<CommandReport>,
    pub test: Option<CommandReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandReport {
    pub command: String,
    pub status_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Error)]
pub enum GitWorktreeError {
    #[error("failed to resolve git repository from `{path}`: {details}")]
    InvalidRepository { path: PathBuf, details: String },
    #[error("io error at `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "git command failed in `{cwd}`: `git {command}` exited with status {status_code}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    )]
    GitCommand {
        cwd: PathBuf,
        command: String,
        status_code: i32,
        stdout: String,
        stderr: String,
    },
    #[error("worktree path `{0}` already exists")]
    WorktreePathExists(PathBuf),
    #[error(
        "{operation} for branch `{source_branch}` against `{target_branch}` failed with a merge conflict:\n{details}"
    )]
    Conflict {
        operation: &'static str,
        source_branch: String,
        target_branch: String,
        details: String,
    },
    #[error(
        "{step} verification failed for `{command}` with status {status_code}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    )]
    VerificationFailed {
        step: &'static str,
        command: String,
        status_code: i32,
        stdout: String,
        stderr: String,
    },
}

impl GitWorktreeOrchestrator {
    pub fn new(repo_path: impl AsRef<Path>) -> Result<Self, GitWorktreeError> {
        let requested_path = repo_path.as_ref().to_path_buf();
        let repo_root_output = run_command(
            Command::new("git")
                .arg("-C")
                .arg(&requested_path)
                .arg("rev-parse")
                .arg("--show-toplevel"),
            &requested_path,
            "rev-parse --show-toplevel".to_string(),
        )
        .map_err(|error| match error {
            GitWorktreeError::GitCommand { stderr, .. } => GitWorktreeError::InvalidRepository {
                path: requested_path.clone(),
                details: stderr,
            },
            other => other,
        })?;

        let repo_root = PathBuf::from(repo_root_output.stdout.trim());
        let repo_name = repo_root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "repo".to_string());
        let parent = repo_root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| repo_root.clone());
        let worktree_root = parent.join(".nexode-worktrees").join(repo_name);

        Ok(Self {
            repo_root,
            worktree_root,
        })
    }

    pub fn with_worktree_root(
        repo_path: impl AsRef<Path>,
        worktree_root: impl AsRef<Path>,
    ) -> Result<Self, GitWorktreeError> {
        let repo_root = Self::new(repo_path)?.repo_root;
        Ok(Self {
            repo_root,
            worktree_root: worktree_root.as_ref().to_path_buf(),
        })
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    pub fn worktree_root(&self) -> &Path {
        &self.worktree_root
    }

    pub fn create_worktree(
        &self,
        slot_id: &str,
        branch_name: &str,
        base_branch: &str,
    ) -> Result<WorktreeHandle, GitWorktreeError> {
        fs::create_dir_all(&self.worktree_root).map_err(|source| GitWorktreeError::Io {
            path: self.worktree_root.clone(),
            source,
        })?;

        let worktree_path = self.worktree_root.join(slot_id);
        if worktree_path.exists() {
            return Err(GitWorktreeError::WorktreePathExists(worktree_path));
        }

        let branch_exists = self.branch_exists(branch_name)?;
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(&self.repo_root)
            .arg("worktree")
            .arg("add");

        if branch_exists {
            command.arg(&worktree_path).arg(branch_name);
        } else {
            command
                .arg("-b")
                .arg(branch_name)
                .arg(&worktree_path)
                .arg(base_branch);
        }

        run_command(
            &mut command,
            &self.repo_root,
            format!(
                "worktree add {} {}",
                worktree_path.display(),
                if branch_exists {
                    branch_name.to_string()
                } else {
                    format!("-b {branch_name} {base_branch}")
                }
            ),
        )?;

        self.describe_worktree(&worktree_path)
    }

    pub fn list_worktrees(&self) -> Result<Vec<WorktreeHandle>, GitWorktreeError> {
        let output = self.run_git(&self.repo_root, ["worktree", "list", "--porcelain"])?;
        let mut worktrees = Vec::new();
        let mut current_path: Option<PathBuf> = None;
        let mut current_branch: Option<String> = None;
        let mut current_head: Option<String> = None;

        for line in output.stdout.lines() {
            if line.is_empty() {
                if let (Some(path), Some(head)) = (current_path.take(), current_head.take()) {
                    worktrees.push(WorktreeHandle {
                        path,
                        branch: current_branch.take(),
                        head,
                    });
                }
                continue;
            }

            if let Some(path) = line.strip_prefix("worktree ") {
                current_path = Some(PathBuf::from(path));
                continue;
            }

            if let Some(head) = line.strip_prefix("HEAD ") {
                current_head = Some(head.to_string());
                continue;
            }

            if let Some(branch) = line.strip_prefix("branch refs/heads/") {
                current_branch = Some(branch.to_string());
            }
        }

        if let (Some(path), Some(head)) = (current_path, current_head) {
            worktrees.push(WorktreeHandle {
                path,
                branch: current_branch,
                head,
            });
        }

        Ok(worktrees)
    }

    pub fn delete_worktree(&self, worktree_path: impl AsRef<Path>) -> Result<(), GitWorktreeError> {
        let worktree_path = worktree_path.as_ref();
        self.run_git(
            &self.repo_root,
            vec![
                OsString::from("worktree"),
                OsString::from("remove"),
                OsString::from("--force"),
                worktree_path.as_os_str().to_os_string(),
            ],
        )?;
        self.run_git(&self.repo_root, ["worktree", "prune"])?;
        Ok(())
    }

    pub fn merge_and_verify(
        &self,
        worktree_path: impl AsRef<Path>,
        target_branch: &str,
        verify: Option<&VerifyConfig>,
    ) -> Result<MergeOutcome, GitWorktreeError> {
        let worktree_path = worktree_path.as_ref();
        let source_branch = self.current_branch(worktree_path)?;

        match self.run_git(
            worktree_path,
            vec![OsString::from("rebase"), OsString::from(target_branch)],
        ) {
            Ok(_) => {}
            Err(GitWorktreeError::GitCommand { stderr, .. }) => {
                let _ = self.run_git(worktree_path, ["rebase", "--abort"]);
                return Err(GitWorktreeError::Conflict {
                    operation: "rebase",
                    source_branch,
                    target_branch: target_branch.to_string(),
                    details: stderr,
                });
            }
            Err(other) => return Err(other),
        }

        let verify_path = self.unique_verify_worktree_path(&source_branch);
        let verification =
            match self.verify_rebased_branch(&verify_path, &source_branch, target_branch, verify) {
                Ok(report) => report,
                Err(error) => {
                    let _ = self.cleanup_verify_worktree(&verify_path);
                    return Err(error);
                }
            };
        self.cleanup_verify_worktree(&verify_path)?;

        self.run_git(
            &self.repo_root,
            vec![OsString::from("switch"), OsString::from(target_branch)],
        )?;
        match self.run_git(
            &self.repo_root,
            vec![
                OsString::from("merge"),
                OsString::from("--ff-only"),
                OsString::from(&source_branch),
            ],
        ) {
            Ok(_) => {}
            Err(GitWorktreeError::GitCommand { stderr, .. }) => {
                return Err(GitWorktreeError::Conflict {
                    operation: "merge",
                    source_branch,
                    target_branch: target_branch.to_string(),
                    details: stderr,
                });
            }
            Err(other) => return Err(other),
        }

        Ok(MergeOutcome {
            source_branch,
            target_branch: target_branch.to_string(),
            verification,
        })
    }

    fn verify_rebased_branch(
        &self,
        verify_path: &Path,
        source_branch: &str,
        target_branch: &str,
        verify: Option<&VerifyConfig>,
    ) -> Result<VerificationReport, GitWorktreeError> {
        fs::create_dir_all(&self.worktree_root).map_err(|source| GitWorktreeError::Io {
            path: self.worktree_root.clone(),
            source,
        })?;
        self.run_git(
            &self.repo_root,
            vec![
                OsString::from("worktree"),
                OsString::from("add"),
                OsString::from("--detach"),
                verify_path.as_os_str().to_os_string(),
                OsString::from(target_branch),
            ],
        )?;

        match self.run_git(
            verify_path,
            vec![
                OsString::from("merge"),
                OsString::from("--ff-only"),
                OsString::from(source_branch),
            ],
        ) {
            Ok(_) => {}
            Err(GitWorktreeError::GitCommand { stderr, .. }) => {
                return Err(GitWorktreeError::Conflict {
                    operation: "verification merge",
                    source_branch: source_branch.to_string(),
                    target_branch: target_branch.to_string(),
                    details: stderr,
                });
            }
            Err(other) => return Err(other),
        }

        let mut report = VerificationReport::default();
        if let Some(verify) = verify {
            if let Some(build) = verify.build.as_deref() {
                report.build = Some(self.run_shell_step(verify_path, "build", build)?);
            }
            if let Some(test) = verify.test.as_deref() {
                report.test = Some(self.run_shell_step(verify_path, "test", test)?);
            }
        }

        Ok(report)
    }

    fn run_shell_step(
        &self,
        cwd: &Path,
        step: &'static str,
        command: &str,
    ) -> Result<CommandReport, GitWorktreeError> {
        let output = Command::new("sh")
            .arg("-lc")
            .arg(command)
            .current_dir(cwd)
            .output()
            .map_err(|source| GitWorktreeError::Io {
                path: cwd.to_path_buf(),
                source,
            })?;

        let report = CommandReport {
            command: command.to_string(),
            status_code: exit_code(output.status),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        };

        if report.status_code != 0 {
            return Err(GitWorktreeError::VerificationFailed {
                step,
                command: report.command.clone(),
                status_code: report.status_code,
                stdout: report.stdout.clone(),
                stderr: report.stderr.clone(),
            });
        }

        Ok(report)
    }

    fn cleanup_verify_worktree(&self, verify_path: &Path) -> Result<(), GitWorktreeError> {
        if verify_path.exists() {
            self.delete_worktree(verify_path)?;
        }
        Ok(())
    }

    fn describe_worktree(&self, worktree_path: &Path) -> Result<WorktreeHandle, GitWorktreeError> {
        let head = self.run_git(worktree_path, ["rev-parse", "HEAD"])?;
        let branch = self
            .run_git(worktree_path, ["branch", "--show-current"])?
            .stdout
            .trim()
            .to_string();

        Ok(WorktreeHandle {
            path: worktree_path.to_path_buf(),
            branch: (!branch.is_empty()).then_some(branch),
            head: head.stdout.trim().to_string(),
        })
    }

    fn current_branch(&self, cwd: &Path) -> Result<String, GitWorktreeError> {
        let branch = self.run_git(cwd, ["branch", "--show-current"])?;
        Ok(branch.stdout.trim().to_string())
    }

    fn branch_exists(&self, branch_name: &str) -> Result<bool, GitWorktreeError> {
        let status = Command::new("git")
            .arg("-C")
            .arg(&self.repo_root)
            .arg("show-ref")
            .arg("--verify")
            .arg("--quiet")
            .arg(format!("refs/heads/{branch_name}"))
            .status()
            .map_err(|source| GitWorktreeError::Io {
                path: self.repo_root.clone(),
                source,
            })?;

        Ok(status.success())
    }

    fn unique_verify_worktree_path(&self, source_branch: &str) -> PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let counter = VERIFY_COUNTER.fetch_add(1, Ordering::Relaxed);
        let sanitized = source_branch.replace('/', "-");
        self.worktree_root
            .join(format!(".verify-{sanitized}-{millis}-{counter}"))
    }

    fn run_git<I, S>(&self, cwd: &Path, args: I) -> Result<CommandOutput, GitWorktreeError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args: Vec<OsString> = args
            .into_iter()
            .map(|arg| arg.as_ref().to_os_string())
            .collect();
        let command_text = args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");

        let mut command = Command::new("git");
        command.current_dir(cwd).args(&args);
        run_command(&mut command, cwd, command_text)
    }
}

#[derive(Debug)]
struct CommandOutput {
    stdout: String,
}

fn run_command(
    command: &mut Command,
    cwd: &Path,
    description: String,
) -> Result<CommandOutput, GitWorktreeError> {
    let output = command.output().map_err(|source| GitWorktreeError::Io {
        path: cwd.to_path_buf(),
        source,
    })?;
    let status_code = exit_code(output.status);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(GitWorktreeError::GitCommand {
            cwd: cwd.to_path_buf(),
            command: description,
            status_code,
            stdout,
            stderr,
        });
    }

    Ok(CommandOutput { stdout })
}

fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::TempDir;

    #[test]
    fn creates_lists_and_deletes_worktrees() {
        let fixture = GitFixture::new();
        let orchestrator = fixture.orchestrator();

        let created = orchestrator
            .create_worktree("slot-a", "agent/slot-a", "main")
            .expect("create worktree");

        assert!(created.path.exists());
        assert_eq!(created.branch.as_deref(), Some("agent/slot-a"));

        let worktrees = orchestrator.list_worktrees().expect("list worktrees");
        assert!(worktrees.iter().any(|entry| entry.path == created.path));
        assert!(worktrees.iter().any(|entry| entry.path == fixture.repo));

        orchestrator
            .delete_worktree(&created.path)
            .expect("delete worktree");
        assert!(!created.path.exists());
    }

    #[test]
    fn merges_rebased_branch_after_successful_verification() {
        let fixture = GitFixture::new();
        let orchestrator = fixture.orchestrator();
        let worktree = orchestrator
            .create_worktree("slot-a", "agent/slot-a", "main")
            .expect("create worktree");

        fixture.write_and_commit(&worktree.path, "app.txt", "slot change\n", "slot change");

        let verify = VerifyConfig {
            build: Some("grep -q 'slot change' app.txt".to_string()),
            test: Some("test -f app.txt".to_string()),
        };

        let outcome = orchestrator
            .merge_and_verify(&worktree.path, "main", Some(&verify))
            .expect("merge and verify");

        assert_eq!(outcome.source_branch, "agent/slot-a");
        assert_eq!(outcome.target_branch, "main");
        assert_eq!(
            fs::read_to_string(fixture.repo.join("app.txt")).expect("read merged file"),
            "slot change\n"
        );
    }

    #[test]
    fn reports_conflicts_without_leaving_rebase_in_progress() {
        let fixture = GitFixture::new();
        let orchestrator = fixture.orchestrator();
        let worktree = orchestrator
            .create_worktree("slot-a", "agent/slot-a", "main")
            .expect("create worktree");

        fixture.write_and_commit(&worktree.path, "app.txt", "slot branch\n", "slot change");
        fixture.write_and_commit(&fixture.repo, "app.txt", "main branch\n", "main change");

        let error = orchestrator
            .merge_and_verify(&worktree.path, "main", None)
            .expect_err("merge should conflict");

        match error {
            GitWorktreeError::Conflict { operation, .. } => assert_eq!(operation, "rebase"),
            other => panic!("expected conflict, got {other:?}"),
        }

        let status = fixture.run_git(&worktree.path, ["status", "--short"]);
        assert!(status.stdout.trim().is_empty());
    }

    #[test]
    fn verification_failure_does_not_update_target_branch() {
        let fixture = GitFixture::new();
        let orchestrator = fixture.orchestrator();
        let worktree = orchestrator
            .create_worktree("slot-a", "agent/slot-a", "main")
            .expect("create worktree");

        fixture.write_and_commit(
            &worktree.path,
            "app.txt",
            "candidate change\n",
            "candidate change",
        );

        let verify = VerifyConfig {
            build: Some("grep -q 'missing' app.txt".to_string()),
            test: None,
        };

        let error = orchestrator
            .merge_and_verify(&worktree.path, "main", Some(&verify))
            .expect_err("verification should fail");

        match error {
            GitWorktreeError::VerificationFailed { step, .. } => assert_eq!(step, "build"),
            other => panic!("expected verification failure, got {other:?}"),
        }

        assert_eq!(
            fs::read_to_string(fixture.repo.join("app.txt")).expect("read main branch file"),
            "base\n"
        );
    }

    struct GitFixture {
        _tempdir: TempDir,
        repo: PathBuf,
        worktrees: PathBuf,
    }

    impl GitFixture {
        fn new() -> Self {
            let tempdir = tempfile::tempdir().expect("tempdir");
            let repo = tempdir.path().join("repo");
            fs::create_dir_all(&repo).expect("create repo dir");

            run_git_in(
                tempdir.path(),
                vec![
                    OsString::from("init"),
                    OsString::from("-b"),
                    OsString::from("main"),
                    repo.as_os_str().to_os_string(),
                ],
            );
            run_git_in(&repo, ["config", "user.email", "test@example.com"]);
            run_git_in(&repo, ["config", "user.name", "Test User"]);
            fs::write(repo.join("app.txt"), "base\n").expect("write initial file");
            run_git_in(&repo, ["add", "."]);
            run_git_in(&repo, ["commit", "-m", "initial"]);

            let worktrees = tempdir.path().join("worktrees");

            Self {
                _tempdir: tempdir,
                repo,
                worktrees,
            }
        }

        fn orchestrator(&self) -> GitWorktreeOrchestrator {
            GitWorktreeOrchestrator::with_worktree_root(&self.repo, &self.worktrees)
                .expect("build orchestrator")
        }

        fn write_and_commit(&self, repo: &Path, file_name: &str, contents: &str, message: &str) {
            fs::write(repo.join(file_name), contents).expect("write file");
            run_git_in(repo, vec![OsString::from("add"), OsString::from(file_name)]);
            run_git_in(
                repo,
                vec![
                    OsString::from("commit"),
                    OsString::from("-m"),
                    OsString::from(message),
                ],
            );
        }

        fn run_git<I, S>(&self, cwd: &Path, args: I) -> CommandReport
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr>,
        {
            let args: Vec<OsString> = args
                .into_iter()
                .map(|arg| arg.as_ref().to_os_string())
                .collect();
            let output = Command::new("git")
                .current_dir(cwd)
                .args(&args)
                .output()
                .expect("run git");
            CommandReport {
                command: args
                    .iter()
                    .map(|arg| arg.to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join(" "),
                status_code: exit_code(output.status),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            }
        }
    }

    fn run_git_in<I, S>(cwd: &Path, args: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let args: Vec<OsString> = args
            .into_iter()
            .map(|arg| arg.as_ref().to_os_string())
            .collect();
        let output = Command::new("git")
            .current_dir(cwd)
            .args(&args)
            .output()
            .expect("run git");
        if !output.status.success() {
            panic!(
                "git {} failed in {}:\nstdout:\n{}\nstderr:\n{}",
                args.iter()
                    .map(|arg| arg.to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join(" "),
                cwd.display(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }
    }
}
