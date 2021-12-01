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

// we want a custom serialization for coords, and so far the cleanest way
// to do this that has been found is to wrap the coord in another struct

//! Data structures and functions to import and export Point of Interests (POIs)
//!
//! POI providers supply data that are transformed into `.poi` files.
//!

use crate::{io, Result};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::collections::{
    btree_map::Entry as BTreeMapEntry, hash_map::Entry as HashMapEntry, BTreeMap, HashMap,
};
use std::path::Path;

/// A thin wrapper around [geo::Coordinate]
#[derive(Debug, Clone, PartialEq)]
pub struct Coord(pub geo::Coordinate<f64>);
impl Coord {
    /// Create a new Coord from longitude and latitude.
    /// Values should be expressed in degrees
    pub fn new(lon: f64, lat: f64) -> Coord {
        Coord(geo::Coordinate { x: lon, y: lat })
    }

    /// Return the longitude
    pub fn lon(&self) -> f64 {
        self.x
    }

    /// Return the latitude
    pub fn lat(&self) -> f64 {
        self.y
    }

    /// Returns true if the latitude and the longitude are
    /// those corresponding to the default values.
    pub fn is_default(&self) -> bool {
        self.lat() == 0. && self.lon() == 0.
    }

    /// Returns true if latitude and longitude are in
    /// a valid range:
    ///
    /// - -90 < lat < 90
    /// - -180 < lon < 180
    pub fn is_valid(&self) -> bool {
        !self.is_default()
            && -90. <= self.lat()
            && self.lat() <= 90.
            && -180. <= self.lon()
            && self.lon() <= 180.
    }
}

impl ::std::ops::Deref for Coord {
    type Target = geo::Coordinate<f64>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Coord {
    fn default() -> Coord {
        Coord(geo::Coordinate { x: 0., y: 0. })
    }
}

impl From<geo::Point<f64>> for Coord {
    fn from(point: geo::Point<f64>) -> Self {
        Coord::new(point.lng(), point.lat())
    }
}

/// A Property of a [Poi]
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Property {
    ///
    /// key
    pub key: String,

    /// value
    pub value: String,
}

/// A Poi
#[derive(Debug, Clone)]
pub struct Poi {
    /// Unique id of the POI
    pub id: String,

    /// Name of the POI
    pub name: String,

    /// Coordinates of the POI
    pub coord: Coord,

    /// The POI type. It is a pointer to a [PoiType]
    pub poi_type_id: String,

    /// List of key values related to the POI
    pub properties: BTreeMap<String, String>,

    /// Indicates if the POI is visible in the map
    pub visible: bool,

    /// Weight
    pub weight: u32,
}

/// A type of POI
#[derive(Debug, Clone, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct PoiType {
    /// Unique id of the POI type
    pub id: String,

    /// Name of the POI type.
    pub name: String,
}

/// A data structure used for exporting and importing data to and from file.
#[derive(Debug, Default)]
pub struct Model {
    /// A list of POIs.
    ///
    /// Could have been a hashmap...
    pub pois: BTreeMap<String, Poi>,

    /// A map of PoiType, indexed by their id.
    ///
    /// We use a hashmap to list poi types, as the main purpose is to search
    /// for a PoiType based on its id. (Poi only stores the type's id)
    pub poi_types: HashMap<String, PoiType>,
}

impl Model {
    /// Creates a new model based on data found in `path`.
    pub fn try_from_path<P: AsRef<Path>>(path: P) -> Result<Model> {
        io::load_model_from_path(path.as_ref())
    }

    /// Saves the model to file.
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        io::write_model_to_path(self, path.as_ref())
    }

    /// Tries to merge a Model into another.
    pub fn try_merge(mut self, rhs: Model) -> Result<Model> {
        let merged_pois = rhs
            .pois
            .into_iter()
            .try_fold(self.pois, |mut acc, (k, v)| match acc.entry(k) {
                BTreeMapEntry::Occupied(entry) => {
                    Err(anyhow!("POI with id {} already in the model", entry.key()))
                }
                BTreeMapEntry::Vacant(entry) => {
                    entry.insert(v);
                    Ok(acc)
                }
            })?;
        self.pois = merged_pois;

        let merged_poi_types =
            rhs.poi_types
                .into_iter()
                .try_fold(self.poi_types, |mut acc, (k, v)| match acc.entry(k) {
                    HashMapEntry::Occupied(entry) => {
                        if *entry.get() == v {
                            Ok(acc) // If the poi_types in both map are identical (id and label), it's ok
                        } else {
                            Err(anyhow!(
                                "Trying to override POI Type with id {}",
                                entry.key()
                            ))
                        }
                    }
                    HashMapEntry::Vacant(entry) => {
                        entry.insert(v);
                        Ok(acc)
                    }
                })?;

        self.poi_types = merged_poi_types;
        Ok(self)
    }
}
