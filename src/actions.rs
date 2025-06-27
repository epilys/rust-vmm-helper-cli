//
// rust-vmm-helper-cli
//
// Copyright 2025 Manos Pitsidianakis <manos.pitsidianakis@linaro.org>
//
// This file is part of rust-vmm-helper-cli.
//
// rust-vmm-helper-cli is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// rust-vmm-helper-cli is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with rust-vmm-helper-cli. If not, see <http://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: EUPL-1.2 OR GPL-3.0-or-later

use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use serde::{Deserialize, Serialize};
use tokio::{io::AsyncWriteExt, process::Command};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub items: Vec<PlanItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    /// Human readable description of item.
    #[serde(default)]
    pub description: Option<String>,
    pub action: Action,
    /// Report any errors but do not halt plan execution.
    #[serde(default)]
    pub continue_on_error: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    /// Interact with <crates.io> HTTP API.
    CratesIO {
        /// Local checkout of repository.
        repository: PathBuf,
        #[serde(flatten)]
        inner: CratesIOAction,
    },
    /// Interact with <github.com> HTTP API.
    Github {
        /// Local checkout of repository.
        repository: PathBuf,
        #[serde(flatten)]
        inner: GithubAction,
    },
    /// Interact with local git repository.
    Local {
        /// Local checkout of repository.
        repository: PathBuf,
        #[serde(flatten)]
        inner: LocalAction,
    },
}

impl Action {
    pub async fn execute(
        &self,
        context: &mut crate::Context,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Self::CratesIO { repository, inner } => inner.execute(repository, context).await,
            Self::Github { repository, inner } => inner.execute(repository, context).await,
            Self::Local { repository, inner } => inner.execute(repository, context).await,
        }
    }

    pub fn as_shell_command(
        &self,
        continue_on_error: bool,
        context: &mut crate::Context,
    ) -> Option<Vec<String>> {
        match self {
            Self::CratesIO { repository, inner } => {
                inner.as_shell_command(continue_on_error, repository, context)
            }
            Self::Github { repository, inner } => {
                inner.as_shell_command(continue_on_error, repository, context)
            }
            Self::Local { repository, inner } => {
                inner.as_shell_command(continue_on_error, repository, context)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CratesIOAction {
    AddOwner {
        crate_name: String,
        login_value: String,
    },
    RemoveOwner {
        crate_name: String,
        login_value: String,
    },
    Publish {
        crate_name: String,
        dry_run: bool,
    },
}

impl CratesIOAction {
    pub async fn execute(
        &self,
        repository: &Path,
        context: &mut crate::Context,
    ) -> Result<(), Box<dyn std::error::Error>> {
        std::env::set_current_dir(repository)?;
        match self {
            Self::AddOwner {
                crate_name,
                login_value,
            } => {
                log::info!("Adding `{login_value}` as owner of crate `{crate_name}`");

                let output = Command::new(context.cargo_bin())
                    .arg("owner")
                    .arg("--add")
                    .arg(login_value)
                    .arg(crate_name)
                    .stdin(Stdio::null())
                    .stderr(Stdio::piped())
                    .stdout(Stdio::piped())
                    .output();
                let output = output.await?;

                if !output.status.success() {
                    return Err(format!(
                        "`cargo owner --add {login_value} {crate_name}` failed: {output:?}"
                    )
                    .into());
                }
                log::info!("OK: Added `{login_value}` as owner of crate `{crate_name}`");
            }
            Self::RemoveOwner {
                crate_name,
                login_value,
            } => {
                log::info!("Removing `{login_value}` as owner of crate `{crate_name}`");

                let output = Command::new(context.cargo_bin())
                    .arg("owner")
                    .arg("--remove")
                    .arg(login_value)
                    .arg(crate_name)
                    .stdin(Stdio::null())
                    .stderr(Stdio::piped())
                    .stdout(Stdio::piped())
                    .output();
                let output = output.await?;

                if !output.status.success() {
                    return Err(format!(
                        "`cargo owner --remove {login_value} {crate_name}` failed: {output:?}"
                    )
                    .into());
                }
                log::info!("OK: Removed `{login_value}` as owner of crate `{crate_name}`");
            }
            Self::Publish {
                crate_name,
                dry_run,
            } => {
                log::info!("Publishing `{crate_name}` [dry_run={dry_run:?}]");

                let mut command = Command::new(context.cargo_bin());
                command.arg("publish").arg("--package").arg(crate_name);
                if *dry_run {
                    command.arg("--dry-run");
                }
                let output = command
                    .stdin(Stdio::null())
                    .stderr(Stdio::piped())
                    .stdout(Stdio::piped())
                    .output();
                let output = output.await?;

                if !output.status.success() {
                    return Err(format!(
                        "`cargo publish --package {crate_name}` failed: {output:?}"
                    )
                    .into());
                }
                log::info!("OK: Publishing `{crate_name}` [dry_run={dry_run:?}]");
            }
        }
        Ok(())
    }

    pub fn as_shell_command(
        &self,
        continue_on_error: bool,
        repository: &Path,
        context: &mut crate::Context,
    ) -> Option<Vec<String>> {
        let cargo_bin = context.cargo_bin();
        let cargo_bin = Path::new(&cargo_bin);
        match self {
            Self::AddOwner {
                crate_name,
                login_value,
            } => Some(vec![
                format!("cd {} || exit 1", repository.display()),
                format!(
                    "{} owner --add {login_value} {crate_name}{continue_on_error}",
                    cargo_bin.display(),
                    continue_on_error = if continue_on_error {
                        " || true"
                    } else {
                        " || exit 1"
                    }
                ),
            ]),
            Self::RemoveOwner {
                crate_name,
                login_value,
            } => Some(vec![
                format!("cd {} || exit 1", repository.display()),
                format!(
                    "{} owner --remove {login_value} {crate_name}{continue_on_error}",
                    cargo_bin.display(),
                    continue_on_error = if continue_on_error {
                        " || true"
                    } else {
                        " || exit 1"
                    }
                ),
            ]),
            Self::Publish {
                crate_name,
                dry_run,
            } => Some(vec![
                format!("cd {} || exit 1", repository.display()),
                format!(
                    "{} publish{dry_run} --package {crate_name}{continue_on_error}",
                    cargo_bin.display(),
                    dry_run = if *dry_run { " --dry-run" } else { "" },
                    continue_on_error = if continue_on_error {
                        " || true"
                    } else {
                        " || exit 1"
                    }
                ),
            ]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GithubAction {
    CreateReleasePR {
        crate_name: String,
        new_version: String,
    },
    CreateRelease {
        crate_name: String,
        tag: String,
        version: String,
    },
}

impl GithubAction {
    pub async fn execute(
        &self,
        repository: &Path,
        context: &mut crate::Context,
    ) -> Result<(), Box<dyn std::error::Error>> {
        std::env::set_current_dir(repository)?;
        match self {
            Self::CreateReleasePR {
                crate_name,
                new_version,
            } => {
                log::info!("Creating release PR for `{crate_name}` v{new_version}`");

                let title = format!("Bump {crate_name} to v{new_version}");
                let output = Command::new(context.gh_bin())
                    .arg("pr")
                    .arg("create")
                    .arg("--title")
                    .arg(title)
                    .arg("--assignee")
                    .arg("@me")
                    .stdin(Stdio::null())
                    .stderr(Stdio::piped())
                    .stdout(Stdio::piped())
                    .output();
                let output = output.await?;

                if !output.status.success() {
                    return Err(format!("`gh pr create` failed: {output:?}").into());
                }
                log::info!("OK: Created PR");
            }
            Self::CreateRelease {
                crate_name,
                tag,
                version,
            } => {
                log::info!("Creating release for `{crate_name}` v{version}`");
                let title = format!("{crate_name}-v{version}");
                let output = Command::new(context.gh_bin())
                    .arg("release")
                    .arg("create")
                    .arg(tag)
                    .arg("--latest")
                    .arg("--notes-from-tag")
                    .arg("--verify-tag")
                    .arg("--title")
                    .arg(title)
                    .stdin(Stdio::null())
                    .stderr(Stdio::piped())
                    .stdout(Stdio::piped())
                    .output();
                let output = output.await?;

                if !output.status.success() {
                    return Err(format!("`gh release create` failed: {output:?}").into());
                }
                log::info!("OK: Created release");
            }
        }
        Ok(())
    }

    pub fn as_shell_command(
        &self,
        _continue_on_error: bool,
        repository: &Path,
        context: &mut crate::Context,
    ) -> Option<Vec<String>> {
        let gh_bin = context.gh_bin();
        let gh_bin = Path::new(&gh_bin);
        match self {
            Self::CreateReleasePR {
                crate_name,
                new_version,
            } => {
                let title = format!("Bump {crate_name} to v{new_version}");
                Some(vec![
                    format!("cd {} || exit 1", repository.display()),
                    format!(
                        "{} pr create --title \"{title}\" --assignee \"@me\"",
                        gh_bin.display()
                    ),
                ])
            }
            Self::CreateRelease {
                crate_name,
                tag,
                version,
            } => {
                let title = format!("{crate_name}-v{version}");
                Some(vec![
                    format!("cd {} || exit 1", repository.display()),
                    format!(
                        "{} release create {tag} --latest --notes-from-tag --verify-tag --title \
                         \"{title}\"",
                        gh_bin.display()
                    ),
                ])
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalAction {
    /// Assert that current checkout is on the default branch.
    AssertDefaultBranch,
    // AssertCargoMetadataVersion {
    //     crate_name: String,
    //     version: String,
    // },
    CreateTags {
        new_tags: Vec<String>,
        // git_ref: Option<String>,
    },
    PublishTags {
        tags: Vec<String>,
        remote: Option<String>,
    },
    // CommitVersionBump {
    //     crate_name: String,
    //     new_version: String,
    // },
    // CommitDependencyUpdate {
    //     crate_name: String,
    //     dependency_name: String,
    //     new_version: String,
    // },
}

impl LocalAction {
    pub async fn execute(
        &self,
        repository: &Path,
        context: &mut crate::Context,
    ) -> Result<(), Box<dyn std::error::Error>> {
        std::env::set_current_dir(repository)?;
        match self {
            Self::AssertDefaultBranch => {
                let repo_name = String::from_utf8(
                    Command::new(context.gh_bin())
                        .args(["repo", "view", "--json", "name", "--jq", ".name"])
                        .stdin(Stdio::null())
                        .stderr(Stdio::piped())
                        .stdout(Stdio::piped())
                        .output()
                        .await?
                        .stdout,
                )
                .unwrap();
                let repo_path = format!("repos/rust-vmm/{repo_name}");
                let default_branch = String::from_utf8(
                    Command::new(context.gh_bin())
                        .args(["api", &repo_path, "--jq", ".default_branch"])
                        .stdin(Stdio::null())
                        .stderr(Stdio::piped())
                        .stdout(Stdio::piped())
                        .output()
                        .await?
                        .stdout,
                )
                .unwrap();
                let current_branch = String::from_utf8(
                    Command::new("git")
                        .args(["rev-parse", "--abbrev-ref", "HEAD"])
                        .stdin(Stdio::null())
                        .stderr(Stdio::piped())
                        .stdout(Stdio::piped())
                        .output()
                        .await?
                        .stdout,
                )
                .unwrap();
                if current_branch != default_branch {
                    return Err(format!(
                        "Currently checked out branch {current_branch} is not the default branch \
                         (`{default_branch}`), please check it out."
                    )
                    .into());
                }
            }
            Self::CreateTags { new_tags } => {
                log::info!("Creating tags");
                if new_tags.is_empty() {
                    log::info!("No tags, doing nothing");
                    return Ok(());
                }
                for tag in new_tags {
                    let mut git_tag = Command::new("git")
                        .arg("tag")
                        .arg("--annotate")
                        .arg("--file")
                        .arg("-")
                        .arg(tag)
                        .stdin(Stdio::piped())
                        .stderr(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()?;
                    let mut stdin = git_tag.stdin.take().unwrap();
                    stdin.write_all(tag.as_bytes()).await?;

                    let output = git_tag.wait_with_output().await?;

                    if !output.status.success() {
                        return Err(format!("`git tag` failed: {output:?}").into());
                    }
                }
                log::info!("OK: created tags");
            }
            Self::PublishTags { tags, remote } => {
                let remote = remote.as_deref().unwrap_or("upstream");
                log::info!("Pushing tags to remote {remote}");
                if tags.is_empty() {
                    log::info!("No tags, doing nothing");
                    return Ok(());
                }
                let output = {
                    let mut cmd = Command::new("git");
                    cmd.arg("push").arg(remote);
                    for tag in tags {
                        cmd.arg(tag);
                    }
                    cmd.stdin(Stdio::null())
                        .stderr(Stdio::piped())
                        .stdout(Stdio::piped())
                        .output()
                };
                let output = output.await?;

                if !output.status.success() {
                    return Err(format!("`git push` failed: {output:?}").into());
                }
                log::info!("OK: Pushed tags to remote {remote:?}");
            }
        }
        Ok(())
    }

    pub fn as_shell_command(
        &self,
        continue_on_error: bool,
        repository: &Path,
        _context: &mut crate::Context,
    ) -> Option<Vec<String>> {
        match self {
            Self::AssertDefaultBranch => Some(vec![
                format!("cd {} || exit 1", repository.display()),
                "repo_name=$(gh repo view --json name --jq \".name\" || exit 1)".into(),
                "default_branch=$(gh api \"repos/rust-vmm/${repo_name}\" --jq '.default_branch' \
                 || exit 1)"
                    .into(),
                "current_branch=$(git rev-parse --abbrev-ref HEAD || exit 1)".into(),
                "[ \"${default_branch}\" != \"${current_branch}\" ] && exit 1".into(),
            ]),
            // Self::AssertCargoMetadataVersion {
            //     crate_name,
            //     version,
            // } => Some(vec![
            //     format!("cd {} || exit 1", repository.display()),
            //     format!(
            //         "cargo_metadata_version=$(cargo metadata --format-version 1 --no-deps| jq
            // '.packages[]|select(.name={crate_name})|.version')"     ),
            //     format!("[ \"${{cargo_metadata_version}}\" != \"{version}\" ] && exit 1")
            // ]),
            Self::CreateTags { new_tags } => {
                let mut cmds = vec![];
                cmds.push(format!("cd {} || exit 1", repository.display()));
                for tag in new_tags {
                    cmds.push(format!("git tag -a {tag} || exit 1"));
                }
                Some(cmds)
            }
            Self::PublishTags { tags, remote } => {
                if tags.is_empty() {
                    return Some(vec![]);
                }
                Some(vec![
                    format!("cd {} || exit 1", repository.display()),
                    format!(
                        "git push {remote} {tags}{continue_on_error}",
                        remote = if let Some(remote) = remote {
                            remote
                        } else {
                            "upstream"
                        },
                        tags = tags.join(" "),
                        continue_on_error = if continue_on_error {
                            " || true"
                        } else {
                            " || exit 1"
                        }
                    ),
                ])
            }
        }
    }
}
