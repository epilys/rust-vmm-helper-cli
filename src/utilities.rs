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

pub mod codeowners {
    use codeowners_rs::RuleSet;

    pub fn from_file(path: &std::path::Path) -> Result<RuleSet, Box<dyn std::error::Error>> {
        let parse_results = codeowners_rs::parse_file(path)?;
        if parse_results.errors.is_empty() {
            return Ok(parse_results.into_ruleset());
        }
        Err(format!(
            "Could not parse {} because of parsing errors: {:?}",
            path.display(),
            parse_results.errors
        )
        .into())
    }
}

pub mod repository {
    use std::path::PathBuf;

    use indexmap::IndexMap;
    use serde_derive::{Deserialize, Serialize};

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Manifest {
        pub package: Package,
        #[serde(default)]
        pub dependencies: IndexMap<String, DependencyField>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Package {
        pub name: String,
        pub version: String,
        #[serde(default)]
        pub edition: Option<String>,
        #[serde(default)]
        pub authors: Vec<String>,
        #[serde(default)]
        pub description: Option<String>,
        #[serde(default)]
        pub repository: Option<String>,
        #[serde(default)]
        pub readme: Option<String>,
        #[serde(default)]
        pub license: Option<String>,
        #[serde(default)]
        pub publish: bool,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum DependencyField {
        Version(String),
        Dependency(Dependency),
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Dependency {
        pub version: String,
        #[serde(default)]
        pub features: Vec<String>,
        #[serde(default)]
        pub path: Option<PathBuf>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Crate {
        pub manifest_path: PathBuf,
        pub manifest: Manifest,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WorkspaceManifest {
        pub workspace: Workspace,
    }
    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Workspace {
        #[serde(default)]
        pub resolver: Option<String>,
        #[serde(default)]
        pub members: Vec<String>,
        #[serde(default)]
        pub exclude: Vec<String>,
    }

    pub fn from_dir(
        package_name: &str,
        path: &std::path::Path,
    ) -> Result<Crate, Box<dyn std::error::Error>> {
        let path = path.canonicalize()?;
        if let Ok(workspace_manifest) =
            toml::from_str::<WorkspaceManifest>(&std::fs::read_to_string(path.join("Cargo.toml"))?)
        {
            for member in workspace_manifest
                .workspace
                .members
                .iter()
                .chain(workspace_manifest.workspace.exclude.iter())
            {
                let manifest_path = path.join(member).join("Cargo.toml");
                log::trace!("for member = {member:?} checking path {manifest_path:?}",);
                if let Ok(manifest) = std::fs::read_to_string(&manifest_path)
                    .map_err(<Box<dyn std::error::Error>>::from)
                    .and_then(|m| toml::from_str::<Manifest>(&m).map_err(|err| err.into()))
                {
                    log::trace!(
                        "found {:?} at path {:?}",
                        manifest.package.name,
                        manifest_path
                    );
                    if manifest.package.name == package_name {
                        return Ok(Crate {
                            manifest_path,
                            manifest,
                        });
                    }
                }
            }
            Err(format!(
                "Could not find package {package_name}, found a workspace with members = {:?} and \
                 exclude = {:?}",
                workspace_manifest.workspace.members, workspace_manifest.workspace.exclude,
            )
            .into())
        } else {
            let manifest_path = path.join("Cargo.toml");
            let manifest = toml::from_str::<Manifest>(&std::fs::read_to_string(&manifest_path)?)?;
            Ok(Crate {
                manifest_path,
                manifest,
            })
        }
    }
}
