// Copyright (C) 2017 Kisio Digital and/or its affiliates.
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published by the
// Free Software Foundation, version 3.

// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.

// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>

//! Data structures and functions to manipulate Points of Interest (POIs)

#![deny(missing_docs, missing_debug_implementations)]

mod io;
pub mod objects;

pub use objects::*;

/// The data type for errors in [navitia-poi-model], just an alias
pub type Error = anyhow::Error;

/// The classic alias for result type.
pub type Result<T> = std::result::Result<T, Error>;
