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

use rust_vmm_helper_cli::{
    Context,
    actions::{Action, CratesIOAction, GithubAction, LocalAction, Plan, PlanItem},
    cli::{ActionCommand, Cli, Command},
    config::CONFIG,
    crates_io::CratesIoAPIQuery,
    health_checks::{self, HealthCheck, HealthCheckError},
    utilities,
};

#[tokio::main]
async fn main() {
    let cli = Cli::new();

    env_logger::Builder::new()
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".to_string()))
        .init();

    log::debug!("Logging initialized.");
    _ = CONFIG.check();

    let mut context = Context::new();

    //let reply = rust_vmm_helper_cli::crates_io::reverse_dependencies::Query {
    //    crate_name: "vm-memory",
    //}
    //.get(&mut context)
    //.await;
    ////log::debug!("API reply was: {:?}", reply);
    //for v in reply.unwrap().unwrap() {
    //    log::debug!("- {:?}", v);
    //}

    //return;
    match cli.command.unwrap_or_else(|| {
        log::info!("No command specified, running health check on current directory.");
        Command::default()
    }) {
        Command::HealthCheck {
            repository,
            package,
            fix,
            json_plan_output,
        } => {
            let crate_name = package.unwrap_or_else(|| todo!());
            let json_plan_output = json_plan_output
                .as_ref()
                .map(|p| (std::fs::File::create(p).unwrap(), p));
            let check_owners = health_checks::CheckCrateOwners {
                crate_name,
                local_crate_path: repository,
            };
            let mut plan_actions = vec![];
            for error in check_owners.exec(&mut context).await.unwrap() {
                let HealthCheckError {
                    description,
                    fix_action,
                } = error;
                let Some(fix_action) = fix_action else {
                    continue;
                };
                plan_actions.push(PlanItem {
                    description: Some(description.clone()),
                    action: fix_action.clone(),
                    continue_on_error: true,
                });
                if !fix {
                    continue;
                }
                log::info!("Fixing `{description}`...");
                fix_action.execute(&mut context).await.unwrap();
                log::info!("Fixed `{description}`.");
            }
            if let Some((mut writer, path)) = json_plan_output {
                log::info!("Serializing plan to `{}`...", path.display());
                serde_json::to_writer_pretty(&mut writer, &Plan {
                    items: plan_actions,
                })
                .expect("Could not serialize plan to file");
                log::info!("Wrote plan to `{}`.", path.display());
            }
        }
        Command::Action {
            action,
            repository,
            json_plan_output,
        } => {
            let repository = repository.canonicalize().unwrap();
            let json_plan_output = json_plan_output
                .as_ref()
                .map(|p| (std::fs::File::create(p).unwrap(), p));
            let mut plan_actions = vec![];
            match action {
                ActionCommand::PublishCrates { package } => {
                    // https://github.com/rust-vmm/community/blob/main/docs/crate_release.md
                    // 1. "Prepare any last-minute changes in a pull request, if necessary." Assume
                    //    this is done already.
                    // 2. "Update the CHANGELOG.md file in the root of the crate's folder. The
                    //    first paragraph should be titled with the version of the new release[..]"
                    //    TODO: use sth like <https://docs.rs/parse-changelog/latest/parse_changelog/>?
                    // 3. "Update the version field in the Cargo.toml file from the crate's root
                    //    folder." TODO: check that it's done already.
                    // 4. "If the crate is part of a workspace and has a path dependency, update
                    //    that dependency in Cargo.toml with a version that is published on
                    //    crates.io as explained in the introduction. This version should be the
                    //    latest one released." TODO
                    let crates_to_publish: Vec<utilities::repository::Crate> = package
                        .iter()
                        .map(|p| utilities::repository::from_dir(p, &repository))
                        .collect::<Result<Vec<_>, _>>()
                        .expect("Could not read repository path");
                    let mut crates: Vec<utilities::repository::Crate> = vec![];
                    for c in crates_to_publish {
                        let reply = rust_vmm_helper_cli::crates_io::get_crate::Query {
                            crate_name: &c.manifest.package.name,
                        }
                        .get(&mut context)
                        .await
                        .unwrap()
                        .unwrap();

                        log::debug!(
                            "get_crate for {:?} API reply was: {:?}",
                            c.manifest.package.name,
                            reply
                        );
                        if c.manifest.package.version != reply.crate_field.default_version {
                            crates.push(c);
                        } else {
                            log::info!(
                                "Skipping package {:?} because local checked out version matches \
                                 published version on crates.io (={:?})",
                                c.manifest.package.name,
                                reply.crate_field.default_version
                            );
                        }
                    }
                    if crates.is_empty() {
                        log::error!("Nothing to do, aborting.");
                        return;
                    }
                    plan_actions.push(PlanItem {
                        description: Some("Ensure we are in default branch".to_string()),
                        action: Action::Local {
                            repository: repository.clone(),
                            inner: LocalAction::AssertDefaultBranch,
                        },
                        continue_on_error: false,
                    });
                    for c in &crates {
                        plan_actions.push(PlanItem {
                            description: Some(format!(
                                "Publish package `{}` to crates.io (dry run)",
                                c.manifest.package.name.clone()
                            )),
                            action: Action::CratesIO {
                                repository: repository.clone(),
                                inner: CratesIOAction::Publish {
                                    crate_name: c.manifest.package.name.clone(),
                                    dry_run: true,
                                },
                            },
                            continue_on_error: false,
                        });
                    }

                    let new_tags: Vec<String> = crates
                        .iter()
                        .map(|c| {
                            format!(
                                "{}-v{}",
                                c.manifest.package.name, c.manifest.package.version
                            )
                        })
                        .collect();
                    assert!(!new_tags.is_empty());
                    plan_actions.push(PlanItem {
                        description: Some(format!("Create tags `{}`", new_tags.join(","))),
                        action: Action::Local {
                            repository: repository.clone(),
                            inner: LocalAction::CreateTags {
                                new_tags: new_tags.clone(),
                            },
                        },
                        continue_on_error: false,
                    });
                    // 7. "Push the tag to the upstream repository: git push upstream
                    //    vm-awesome-v1.2.0. In this example, the upstream remote points to the
                    //    original repository (not your fork)."
                    plan_actions.push(PlanItem {
                        description: Some(format!("Push tags `{}` to remote", new_tags.join(","))),
                        action: Action::Local {
                            repository: repository.clone(),
                            inner: LocalAction::PublishTags {
                                tags: new_tags.clone(),
                                remote: None,
                            },
                        },
                        continue_on_error: false,
                    });
                    // 8. "Create a GitHub release. Go to the Releases page in the crate's
                    //    repository and click Draft a new release (button on the right). In Tag
                    //    version, pick the newly pushed tag. In Release title, write the tag name
                    //    including v (example: vm-awesome-v1.2.3). The description should be the
                    //    new version's changelog section. Click Publish release."
                    for (c, tag) in crates.iter().zip(new_tags.iter()) {
                        plan_actions.push(PlanItem {
                            description: Some(format!(
                                "Create a GitHub release for crate `{}` v{}",
                                c.manifest.package.name.clone(),
                                c.manifest.package.version.clone(),
                            )),
                            action: Action::Github {
                                repository: repository.clone(),
                                inner: GithubAction::CreateRelease {
                                    crate_name: c.manifest.package.name.clone(),
                                    tag: tag.clone(),
                                    version: c.manifest.package.version.clone(),
                                },
                            },
                            continue_on_error: false,
                        });
                    }
                    // 9. "Publish the new version to crates.io. To double-check what's being
                    //    published, do a dry run first. Make sure your HEAD is on the release tag."
                    for c in &crates {
                        plan_actions.push(PlanItem {
                            description: Some(format!(
                                "Publish package `{}` to crates.io",
                                c.manifest.package.name.clone()
                            )),
                            action: Action::CratesIO {
                                repository: repository.clone(),
                                inner: CratesIOAction::Publish {
                                    crate_name: c.manifest.package.name.clone(),
                                    dry_run: false,
                                },
                            },
                            continue_on_error: false,
                        });
                    }
                }
            }
            if let Some((mut writer, path)) = json_plan_output {
                log::info!("Serializing plan to `{}`...", path.display());
                serde_json::to_writer_pretty(&mut writer, &Plan {
                    items: plan_actions,
                })
                .expect("Could not serialize plan to file");
                log::info!("Wrote plan to `{}`.", path.display());
            }
        }
        Command::ExecuteActionPlan {
            json_plan_input,
            shellscript_output,
            dry_run,
        } => {
            let plan: Plan = if let Some(path) = json_plan_input {
                log::info!("Reading action plan from `{}`...", path.display());
                let file = std::fs::File::open(path).unwrap();
                let reader = std::io::BufReader::new(file);
                serde_json::from_reader(reader).unwrap()
            } else {
                log::info!("Reading action plan from stdin...");
                serde_json::from_reader(std::io::stdin().lock()).unwrap()
            };

            log::debug!("Parsed action plan: {plan:?}");

            if let Some(shellscript_output) = shellscript_output {
                use std::io::Write;

                let mut f = std::fs::File::create(&shellscript_output).unwrap();
                log::info!(
                    "Serializing plan to shell script `{}`...",
                    shellscript_output.display()
                );
                writeln!(&mut f, "#!/bin/sh\n\nset -ev\n").unwrap();
                // Do not repeat redundant directory changes
                let mut prev_cd = None;
                for plan_item in plan.items {
                    if let Some(ref description) = plan_item.description {
                        writeln!(&mut f, "# {description}").unwrap();
                    }
                    let Some(cmds) = plan_item
                        .action
                        .as_shell_command(plan_item.continue_on_error, &mut context)
                    else {
                        log::error!(
                            "Can not represent plan item {plan_item:?} as shell script, aborting."
                        );
                        return;
                    };
                    for cmd in cmds {
                        if matches!(prev_cd.as_ref(), Some(prev) if prev == &cmd) {
                            continue;
                        }
                        writeln!(&mut f, "{cmd}").unwrap();
                        if cmd.starts_with("cd ") {
                            prev_cd = Some(cmd);
                        }
                    }
                }
                f.flush().unwrap();
                drop(f);
                log::info!(
                    "Wrote plan shell script to `{}`.",
                    shellscript_output.display()
                );
            } else if !dry_run {
                for plan_item in plan.items {
                    if let Some(ref description) = plan_item.description {
                        log::info!("Executing `{description}`...");
                    } else {
                        log::info!("Executing `{:?}`...", plan_item.action);
                    }
                    let result = plan_item.action.execute(&mut context).await;
                    if result.is_err() && !plan_item.continue_on_error {
                        result.unwrap();
                    } else if let Err(err) = result {
                        log::error!("Action failed: {err}");
                        log::info!("Continuing");
                    }
                    if let Some(ref description) = plan_item.description {
                        log::info!("Executed `{description}`.");
                    } else {
                        log::info!("Executed `{:?}`.", plan_item.action);
                    }
                }
            }
        }
    }
}
