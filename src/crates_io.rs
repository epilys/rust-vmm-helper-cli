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
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CratesIoAPIErrorDetail {
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CratesIoAPIError {
    pub errors: Vec<CratesIoAPIErrorDetail>,
}

#[async_trait]
pub trait CratesIoAPIQuery {
    type Response;

    async fn get(
        &self,
        context: &mut crate::Context,
    ) -> Result<Result<Self::Response, CratesIoAPIError>, Box<dyn std::error::Error>>;
}

pub trait CratesIoAPIResponse: for<'a> serde::Deserialize<'a> {
    fn try_deserialize(
        body: &[u8],
    ) -> Result<Result<Self, CratesIoAPIError>, Box<dyn std::error::Error>> {
        {
            let de = &mut serde_json::Deserializer::from_slice(body);
            if let Ok(error_reply) = CratesIoAPIError::deserialize(de) {
                return Ok(Err(error_reply));
            }
        }
        let de = &mut serde_json::Deserializer::from_slice(body);
        Ok(Ok(serde_path_to_error::deserialize(de)?))
    }
}

pub mod get_owners {
    use super::*;

    pub struct Query<'a> {
        pub crate_name: &'a str,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Response {
        pub users: Vec<User>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct User {
        pub id: i64,
        pub login: String,
        pub kind: String,
        pub url: String,
        pub name: String,
        pub avatar: String,
    }

    #[async_trait]
    impl<'a> CratesIoAPIQuery for Query<'a> {
        type Response = Response;

        async fn get(
            &self,
            context: &mut crate::Context,
        ) -> Result<Result<Self::Response, CratesIoAPIError>, Box<dyn std::error::Error>> {
            let octocrab = octocrab::instance();
            let owners_endpoint =
                format!("https://crates.io/api/v1/crates/{}/owners", self.crate_name);
            context.crates_io_call().await;
            let response = octocrab._get(&owners_endpoint).await?;
            log::debug!("response: {:?}", response);
            let (_parts, body) = response.into_parts();
            let owners_reply = body.collect().await?.to_bytes();
            log::debug!("API reply was: {:?}", owners_reply);
            Self::Response::try_deserialize(&owners_reply)
        }
    }

    impl CratesIoAPIResponse for Response {}
}

pub mod get_crate {
    use super::*;

    pub struct Query<'a> {
        pub crate_name: &'a str,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Response {
        #[serde(rename = "crate")]
        pub crate_field: Crate,
        #[serde(skip)]
        pub versions: (),
        #[serde(skip)]
        pub keywords: (),
        #[serde(skip)]
        pub categories: (),
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Crate {
        pub id: String,
        pub name: String,
        #[serde(rename = "updated_at")]
        pub updated_at: String,
        pub versions: Value,
        #[serde(skip)]
        pub keywords: (),
        #[serde(skip)]
        pub categories: (),
        #[serde(skip)]
        pub badges: (),
        #[serde(rename = "created_at")]
        pub created_at: String,
        pub downloads: i64,
        #[serde(rename = "recent_downloads", default)]
        pub recent_downloads: Option<i64>,
        #[serde(rename = "default_version")]
        pub default_version: String,
        #[serde(rename = "num_versions")]
        pub num_versions: i64,
        pub yanked: bool,
        #[serde(rename = "max_version")]
        pub max_version: String,
        #[serde(rename = "newest_version")]
        pub newest_version: String,
        #[serde(rename = "max_stable_version")]
        pub max_stable_version: Value,
        #[serde(default)]
        pub description: Option<String>,
        #[serde(default)]
        pub homepage: Value,
        #[serde(default)]
        pub documentation: Option<String>,
        #[serde(default)]
        pub repository: Option<String>,
        #[serde(skip)]
        pub links: (),
        #[serde(rename = "exact_match")]
        pub exact_match: bool,
    }

    #[async_trait]
    impl<'a> CratesIoAPIQuery for Query<'a> {
        type Response = Response;

        async fn get(
            &self,
            context: &mut crate::Context,
        ) -> Result<Result<Self::Response, CratesIoAPIError>, Box<dyn std::error::Error>> {
            let octocrab = octocrab::instance();
            let crate_endpoint = format!(
                "https://crates.io/api/v1/crates/{}?include=default_version",
                self.crate_name
            );
            context.crates_io_call().await;
            let response = octocrab._get(&crate_endpoint).await?;
            log::debug!("response: {:?}", response);
            let (_parts, body) = response.into_parts();
            let crate_reply = body.collect().await?.to_bytes();
            log::debug!("API reply was: {:?}", crate_reply);
            Self::Response::try_deserialize(&crate_reply)
        }
    }

    impl CratesIoAPIResponse for Response {}
}

// GET https://crates.io/api/v1/crates/libloading/reverse_dependencies?page=2&per_page=10
pub mod reverse_dependencies {
    use super::*;

    pub struct Query<'a> {
        pub crate_name: &'a str,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Response {
        pub dependencies: Vec<Dependency>,
        pub versions: Vec<Version>,
        pub meta: Meta,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Dependency {
        pub id: i64,
        #[serde(rename = "version_id")]
        pub version_id: i64,
        #[serde(rename = "crate_id")]
        pub crate_id: String,
        pub req: String,
        pub optional: bool,
        #[serde(rename = "default_features")]
        pub default_features: bool,
        #[serde(skip)]
        pub features: (),
        pub target: Option<String>,
        pub kind: String,
        pub downloads: i64,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Version {
        pub id: i64,
        #[serde(rename = "crate")]
        pub crate_field: String,
        pub num: String,
        #[serde(rename = "dl_path")]
        pub dl_path: String,
        #[serde(rename = "readme_path")]
        pub readme_path: String,
        #[serde(rename = "updated_at")]
        pub updated_at: String,
        #[serde(rename = "created_at")]
        pub created_at: String,
        pub downloads: i64,
        #[serde(skip)]
        pub features: (),
        pub yanked: bool,
        #[serde(rename = "yank_message", skip)]
        pub yank_message: (),
        #[serde(rename = "lib_links")]
        pub lib_links: Value,
        pub license: String,
        #[serde(skip)]
        pub links: (),
        #[serde(rename = "crate_size")]
        pub crate_size: i64,
        #[serde(rename = "published_by", skip)]
        pub published_by: (),
        #[serde(rename = "audit_actions", skip)]
        pub audit_actions: (),
        pub checksum: String,
        #[serde(rename = "rust_version")]
        pub rust_version: Option<String>,
        #[serde(rename = "has_lib")]
        pub has_lib: bool,
        #[serde(rename = "bin_names")]
        pub bin_names: Vec<String>,
        pub edition: String,
        pub description: String,
        pub homepage: Option<String>,
        pub documentation: Option<String>,
        pub repository: Option<String>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct User {
        pub id: i64,
        pub login: String,
        pub name: Option<String>,
        pub avatar: String,
        pub url: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Meta {
        pub total: usize,
    }

    #[async_trait]
    impl<'a> CratesIoAPIQuery for Query<'a> {
        type Response = Vec<Version>;

        async fn get(
            &self,
            context: &mut crate::Context,
        ) -> Result<Result<Self::Response, CratesIoAPIError>, Box<dyn std::error::Error>> {
            let octocrab = octocrab::instance();
            let mut page = 1;
            let mut accumulator = vec![];
            loop {
                let endpoint =
                    format!("https://crates.io/api/v1/crates/{}/reverse_dependencies?page={page}&per_page=10", self.crate_name);
                context.crates_io_call().await;
                let response = octocrab._get(&endpoint).await?;
                log::debug!("response: {response:?}");
                let (_parts, body) = response.into_parts();
                let reply = body.collect().await?.to_bytes();
                log::debug!("API reply was: {reply:?}");
                let response = Response::try_deserialize(&reply)?;
                let Response {
                    dependencies: _,
                    versions,
                    meta,
                } = match response {
                    Ok(v) => v,
                    Err(err) => return Ok(Err(err)),
                };
                accumulator.extend(versions);
                if accumulator.len() >= meta.total {
                    break;
                }
                page += 1;
            }
            accumulator.retain(|v| {
                matches!(
                    v.repository,
                    Some(ref repository) if repository.starts_with("https://github.com/rust-vmm")
                )
            });
            Ok(Ok(accumulator))
        }
    }

    impl CratesIoAPIResponse for Response {}
}
