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

use async_trait::async_trait;

use crate::{
    actions::{Action, CratesIOAction},
    crates_io::CratesIoAPIQuery,
};

#[derive(Debug)]
pub struct HealthCheckError {
    pub description: String,
    pub fix_action: Option<Action>,
}

#[async_trait]
pub trait HealthCheck {
    async fn exec(
        &self,
        context: &mut crate::Context,
    ) -> Result<Vec<HealthCheckError>, Box<dyn std::error::Error>>;
}

#[derive(Debug)]
pub struct CheckCrateOwners {
    pub crate_name: String,
    pub local_crate_path: std::path::PathBuf,
}

#[async_trait]
impl HealthCheck for CheckCrateOwners {
    async fn exec(
        &self,
        context: &mut crate::Context,
    ) -> Result<Vec<HealthCheckError>, Box<dyn std::error::Error>> {
        log::info!("Running {:?}", self);
        let codeowners = crate::utilities::codeowners::from_file(
            &self.local_crate_path.as_path().join("CODEOWNERS"),
        )?;

        log::debug!("codeowners: {:?}", codeowners.all_matching_rules("."));
        let reply = crate::crates_io::get_owners::Query {
            crate_name: &self.crate_name,
        }
        .get(context)
        .await?;
        log::debug!("API reply was: {:?}", reply);
        let mut results = vec![];
        match reply {
            Ok(owners) => {
                log::debug!("owners: {owners:?}");
                if !owners
                    .users
                    .iter()
                    .any(|owner| owner.login == "github:rust-vmm:gatekeepers")
                {
                    let description = format!(
                        "rust-vmm:gatekeepers team must be in {:?}'s owners.",
                        self.crate_name
                    );
                    log::error!("{description}");
                    results.push(HealthCheckError {
                        description,
                        fix_action: Some(Action::CratesIO {
                            repository: self.local_crate_path.clone(),
                            inner: CratesIOAction::AddOwner {
                                crate_name: self.crate_name.clone(),
                                login_value: "rust-vmm:gatekeepers".into(),
                            },
                        }),
                    });
                } else {
                    log::info!("OK: rust-vmm:gatekeepers is an owner.");
                }
                if let Some(repo_code_owners) = codeowners.owners(".") {
                    for repo_code_owner in repo_code_owners {
                        let repo_code_owner = repo_code_owner.value.trim_start_matches("@");
                        if !owners
                            .users
                            .iter()
                            .any(|owner| owner.login == repo_code_owner)
                        {
                            let description = format!(
                                "{} user must be in {:?}'s owners.",
                                repo_code_owner, self.crate_name
                            );
                            log::error!("{description}");
                            results.push(HealthCheckError {
                                description,
                                fix_action: Some(Action::CratesIO {
                                    repository: self.local_crate_path.clone(),
                                    inner: CratesIOAction::AddOwner {
                                        crate_name: self.crate_name.clone(),
                                        login_value: repo_code_owner.into(),
                                    },
                                }),
                            });
                        } else {
                            log::info!("OK: {repo_code_owner} is an owner.");
                        }
                    }
                }
            }
            Err(err) if err.errors.len() == 1 && err.errors[0].detail == "Not Found" => {
                return Err(format!(
                    "Crate {} was not found on crates.io and is assumed unpublished.",
                    self.crate_name
                )
                .into());
            }
            Err(other_err) => {
                return Err(format!("crates.io error: {other_err:?}").into());
            }
        }
        Ok(results)
    }
}
