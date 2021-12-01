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

use crate::Result;
use crate::{Coord, Model, Poi, PoiType, Property};
use anyhow::anyhow;
use itertools::Itertools;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::path::Path;

/// Saves the model to a file, in CSV format.
pub fn write_model_to_path<P>(model: &Model, path: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let out = path.as_ref().with_extension("poi");
    let file = File::create(out)?;
    let mut zip = zip::ZipWriter::new(file);

    zip.start_file("poi.txt", zip::write::FileOptions::default())?;

    write_csv(
        &mut zip,
        model.pois.iter().map(|(_, poi)| PoiRecord::from(poi)),
    )?;

    zip.start_file("poi_type.txt", zip::write::FileOptions::default())?;

    write_csv(
        &mut zip,
        model
            .poi_types
            .iter()
            .sorted_by_key(|pt| pt.0)
            .map(|pt| PoiTypeRecord::from(pt.1.clone())),
    )?;

    zip.start_file("poi_properties.txt", zip::write::FileOptions::default())?;

    let poi_properties = model.pois.values().flat_map(|poi| {
        poi.properties.iter().map(move |(k, v)| PoiProperty {
            poi_id: poi.id.clone(),
            key: k.to_string(),
            value: v.to_string(),
        })
    });
    write_csv(&mut zip, poi_properties)?;

    Ok(())
}

/// Takes a zipped file containing pois, types, and properties,
/// and returns the corresponding model
pub fn load_model_from_path<P>(path: P) -> Result<Model>
where
    P: AsRef<Path>,
{
    let file = File::open(path.as_ref())?;
    let mut zip = zip::ZipArchive::new(file)?;

    let mut pois: BTreeMap<String, Poi> = {
        let zipper = zip.by_name("poi.txt")?;
        let reader = read_csv(zipper);
        reader
            .map(|rec| {
                let rec: PoiRecord = rec?;
                let poi = Poi::from(rec);
                Ok((poi.id.clone(), poi))
            })
            .collect::<Result<_>>()?
    };
    let poi_types: HashMap<String, PoiType> = {
        let zipper = zip.by_name("poi_type.txt")?;
        let reader = read_csv(zipper);
        reader
            .map(|rec| {
                let poi_type_rec: PoiTypeRecord = rec?;
                let poi_type = PoiType::from(poi_type_rec);
                Ok((poi_type.id.clone(), poi_type))
            })
            .collect::<Result<_>>()?
    };
    // For poi_properties.txt, it's a bit different: If the file is not
    // present, it does not mean it is an error.
    if let Ok(zipper) = zip.by_name("poi_properties.txt") {
        read_csv(zipper).try_for_each::<_, Result<_>>(|rec| {
            let poi_property: PoiProperty = rec?;
            let poi = pois.get_mut(&poi_property.poi_id).ok_or_else(|| {
                anyhow!(
                    "in file '{}', cannot find poi '{}' for property insertion",
                    path.as_ref().display(),
                    &poi_property.poi_id
                )
            })?;
            poi.properties.insert(poi_property.key, poi_property.value);
            Ok(())
        })?;
    }
    Ok(Model { pois, poi_types })
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn ser_from_bool<S>(v: &bool, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u8(*v as u8)
}

fn de_from_u8<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let i = u8::deserialize(deserializer)?;
    Ok(i != 0)
}

/// Used to import / export POI to / from CSV
#[derive(Debug, Deserialize, Serialize)]
pub struct PoiRecord {
    #[serde(rename = "poi_id")]
    pub id: String,
    #[serde(rename = "poi_type_id")]
    pub type_id: String,
    #[serde(rename = "poi_name")]
    pub name: String,
    #[serde(rename = "poi_lat")]
    pub lat: f64,
    #[serde(rename = "poi_lon")]
    pub lon: f64,
    #[serde(rename = "poi_weight")]
    pub weight: u32,
    #[serde(
        rename = "poi_visible",
        serialize_with = "ser_from_bool",
        deserialize_with = "de_from_u8"
    )]
    pub visible: bool,
}

impl From<&Poi> for PoiRecord {
    fn from(poi: &Poi) -> PoiRecord {
        PoiRecord {
            id: poi.id.clone(),
            type_id: poi.poi_type_id.clone(),
            name: poi.name.clone(),
            lat: poi.coord.lat(),
            lon: poi.coord.lon(),
            visible: poi.visible,
            weight: poi.weight,
        }
    }
}

impl From<PoiRecord> for Poi {
    fn from(record: PoiRecord) -> Poi {
        Poi {
            id: record.id,
            name: record.name,
            coord: Coord::new(record.lon, record.lat),
            poi_type_id: record.type_id,
            properties: BTreeMap::default(),
            visible: record.visible,
            weight: record.weight,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq)]
struct PoiProperty {
    pub poi_id: String,
    pub key: String,
    pub value: String,
}

impl From<PoiProperty> for Property {
    fn from(property: PoiProperty) -> Property {
        Property {
            key: property.key,
            value: property.value,
        }
    }
}

/// A type of POI
/// We use a different type for serialization, because we want to make sure
/// we have adequate headers. In some files (json?) the headers are just
/// 'id', and 'name', while in others it's 'poi_type_id' and 'poi_type_name'
#[derive(Debug, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct PoiTypeRecord {
    /// Unique id of the POI type
    #[serde(rename = "poi_type_id")]
    pub id: String,

    /// Name of the POI type.
    #[serde(rename = "poi_type_name")]
    pub name: String,
}

impl From<PoiTypeRecord> for PoiType {
    fn from(record: PoiTypeRecord) -> PoiType {
        PoiType {
            id: record.id,
            name: record.name,
        }
    }
}

impl From<PoiType> for PoiTypeRecord {
    fn from(poi_type: PoiType) -> PoiTypeRecord {
        PoiTypeRecord {
            id: poi_type.id,
            name: poi_type.name,
        }
    }
}

/// Converts items into CSV, and streams them to a writer.
fn write_csv<W, I, T>(writer: W, items: I) -> Result<()>
where
    W: std::io::Write,
    I: Iterator<Item = T>,
    T: Serialize,
{
    let mut csv_writer = csv::WriterBuilder::new()
        .has_headers(true)
        .delimiter(b';')
        .from_writer(writer);
    for item in items {
        csv_writer.serialize(item)?;
    }
    Ok(())
}

/// Streams records from a CSV
fn read_csv<R, T>(reader: R) -> impl Iterator<Item = Result<T>>
where
    R: std::io::Read,
    T: DeserializeOwned,
{
    let csv_reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_reader(reader);
    csv_reader
        .into_deserialize()
        .map(|e| e.map_err(|e| anyhow!("err {}", e)))
}
