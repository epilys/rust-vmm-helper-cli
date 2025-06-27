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

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Override cargo binary location, otherwise the one from `PATH` is used.
    #[arg(long, value_name = "CARGO_BIN")]
    pub cargo_bin: Option<PathBuf>,

    /// Default command is `health-check`.
    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Default for Cli {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Perform health checks on a repository.
    HealthCheck {
        /// Path to local checkout of repository.
        #[arg(short, long, value_name = "REPO_PATH")]
        repository: PathBuf,
        /// Check only one package in repository instead of all.
        #[arg(short, long, value_name = "NAME")]
        package: Option<String>,
        /// Attempt to fix automatically.
        #[arg(short, long, default_value = "false")]
        fix: bool,
        /// Save serialized errorfix plan output to file.
        #[arg(short, long, value_name = "JSON_PLAN_FILE")]
        json_plan_output: Option<PathBuf>,
    },
    /// Attempts to generate an action plan into a JSON file.
    Action {
        /// Action to execute
        #[command(subcommand)]
        action: ActionCommand,
        /// Path to local checkout of repository.
        #[arg(short, long, value_name = "REPO_PATH")]
        repository: PathBuf,
        /// Save serialized action plan output to file.
        #[arg(short, long, value_name = "JSON_PLAN_FILE")]
        json_plan_output: Option<PathBuf>,
    },
    /// Executes serialized action plan JSON.
    ExecuteActionPlan {
        /// Read action plan from file instead of `STDIN`.
        #[arg(short, long, value_name = "JSON_PLAN_FILE")]
        json_plan_input: Option<PathBuf>,
        /// Output shell script instead of executing it.
        #[arg(short, long, value_name = "SHELLSCRIPT_OUTPUT_FILE")]
        shellscript_output: Option<PathBuf>,
        /// Dry run (validate but do not actually perform anything).
        #[arg(short, long, default_value = "false")]
        dry_run: bool,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ActionCommand {
    /// Publish one or more crates to <crates.io>.
    PublishCrates {
        /// Package(s) to publish
        package: Vec<String>,
    },
}

impl Default for Command {
    fn default() -> Self {
        Self::HealthCheck {
            repository: std::env::current_dir()
                .expect("Command::default(): Could not access current process directory"),
            package: None,
            fix: false,
            json_plan_output: None,
        }
    }
}

impl Cli {
    pub fn new() -> Self {
        Self::parse()
    }
}
