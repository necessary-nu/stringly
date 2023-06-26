//! Intermediate representation of our localisation project:
//!
//! InputData -> ProjectData -> StringMap -> StringData

use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Project(pub BTreeMap<String, Category>);

impl Project {
    pub fn into_inner(self) -> BTreeMap<String, Category> {
        self.0
    }
}

impl Deref for Project {
    type Target = BTreeMap<String, Category>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Project {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone)]
pub struct Category {
    pub base_language: String,
    pub translation_units: BTreeMap<String, TranslationUnitMap>,
}

impl Category {
    pub fn base_strings(&self) -> &TranslationUnitMap {
        self.translation_units.get(&self.base_language).unwrap()
    }
}

impl Deref for Category {
    type Target = BTreeMap<String, TranslationUnitMap>;

    fn deref(&self) -> &Self::Target {
        &self.translation_units
    }
}

impl DerefMut for Category {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.translation_units
    }
}

#[derive(Debug, Clone)]
pub struct TranslationUnitMap {
    pub language: String,
    pub translation_units: BTreeMap<String, TranslationUnit>,
}

impl Deref for TranslationUnitMap {
    type Target = BTreeMap<String, TranslationUnit>;

    fn deref(&self) -> &Self::Target {
        &self.translation_units
    }
}

impl DerefMut for TranslationUnitMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.translation_units
    }
}

#[derive(Debug, Clone)]
pub struct TranslationUnit {
    pub main: String,
    pub attributes: BTreeMap<String, String>,
}
