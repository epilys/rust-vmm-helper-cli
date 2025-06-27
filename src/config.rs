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

lazy_static::lazy_static! {
    pub static ref CONFIG: Config = {
        match Config::init() {
            Ok(c) => {
                log::info!("Configuration parsed from environment variables.");
                c
            }
            Err(missing) => {
                log::error!("Unable to load environment variables {missing:?}");
                panic!("Unable to load environment variables {missing:?}");
            }
        }
    };
}

#[derive(Debug)]
pub struct Config {}

impl Config {
    pub fn check(&self) -> bool {
        true
    }

    fn init() -> Result<Self, String> {
        Ok(Self {})
    }
}
