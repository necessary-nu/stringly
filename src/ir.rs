//! Intermediate representation of our localisation project:
//!
//! InputData -> ProjectData -> StringMap -> StringData

use std::{collections::BTreeMap, ops::Deref};

#[derive(Debug)]
#[repr(transparent)]
pub struct InputData(pub BTreeMap<String, ProjectData>);

impl InputData {
    pub fn into_inner(self) -> BTreeMap<String, ProjectData> {
        self.0
    }
}

impl Deref for InputData {
    type Target = BTreeMap<String, ProjectData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct ProjectData {
    pub base_language: String,
    pub strings: BTreeMap<String, StringMap>,
}

impl ProjectData {
    pub fn base_strings(&self) -> &StringMap {
        self.strings.get(&self.base_language).unwrap()
    }
}

#[derive(Debug)]
pub struct StringData {
    pub base: String,
    pub meta: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct StringMap {
    pub language: String,
    pub strings: BTreeMap<String, StringData>,
}
