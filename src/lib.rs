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

use std::time::{Duration, Instant};

pub mod actions;
pub mod cli;
pub mod config;
pub mod crates_io;
pub mod health_checks;
pub mod utilities;

#[derive(Debug)]
pub struct Context {
    cargo_bin: Option<std::path::PathBuf>,
    gh_bin: Option<std::path::PathBuf>,
    last_crates_io_call: Instant,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    pub fn new() -> Self {
        let last_crates_io_call = Instant::now() - Duration::from_secs(3);
        Self {
            cargo_bin: None,
            gh_bin: None,
            last_crates_io_call,
        }
    }

    pub fn cargo_bin(&self) -> impl AsRef<std::ffi::os_str::OsStr> {
        self.cargo_bin
            .as_deref()
            .map(|p| p.as_ref())
            .unwrap_or_else(|| std::ffi::OsStr::new("cargo"))
    }

    pub fn gh_bin(&self) -> impl AsRef<std::ffi::os_str::OsStr> {
        self.gh_bin
            .as_deref()
            .map(|p| p.as_ref())
            .unwrap_or_else(|| std::ffi::OsStr::new("gh"))
    }

    pub async fn crates_io_call(&mut self) {
        const ONE_SECOND: Duration = Duration::from_secs(1);

        if self.last_crates_io_call.elapsed() >= ONE_SECOND {
            self.last_crates_io_call = Instant::now();
        } else {
            tokio::time::sleep(Duration::from_secs(1)).await;
            self.last_crates_io_call = Instant::now();
        }
    }
}
