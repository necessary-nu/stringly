//! Intermediate representation of our localisation project:
//!
//! Project -> Category -> TranslationUnitMap -> TranslationUnit

use std::{
    collections::BTreeMap,
    convert::Infallible,
    fmt::Display,
    ops::{Deref, DerefMut},
    str::FromStr,
};

use icu::locid::Locale;

use crate::BTreeKeyedSet;

#[derive(Debug, Clone)]
pub struct Project {
    pub categories: BTreeKeyedSet<CIdentifier, Category>,
}

impl Deref for Project {
    type Target = BTreeKeyedSet<CIdentifier, Category>;

    fn deref(&self) -> &Self::Target {
        &self.categories
    }
}

impl DerefMut for Project {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.categories
    }
}

#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
    pub base_locale: Locale,
    pub translation_units: BTreeKeyedSet<Locale, TranslationUnitMap>,
}

impl Category {
    pub fn base_strings(&self) -> &TranslationUnitMap {
        self.translation_units.get(&self.base_locale).unwrap()
    }
}

impl Deref for Category {
    type Target = BTreeKeyedSet<Locale, TranslationUnitMap>;

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
    pub locale: Locale,
    pub translation_units: BTreeMap<TUIdentifier, TranslationUnit>,
}

impl Eq for TranslationUnitMap {}

impl PartialEq for TranslationUnitMap {
    fn eq(&self, other: &Self) -> bool {
        self.locale == other.locale
    }
}

impl PartialOrd for TranslationUnitMap {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.locale.partial_cmp(&other.locale) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        Some(core::cmp::Ordering::Equal)
    }
}

impl Ord for TranslationUnitMap {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.locale.cmp(&other.locale)
    }
}

impl Deref for TranslationUnitMap {
    type Target = BTreeMap<TUIdentifier, TranslationUnit>;

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

// TODO: validate the identifier is a valid FLT identifier plus optional attribute
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TUIdentifier(String);

impl Deref for TUIdentifier {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for TUIdentifier {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl TryFrom<String> for TUIdentifier {
    type Error = Infallible;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(TUIdentifier(value))
    }
}

impl TryFrom<&String> for TUIdentifier {
    type Error = Infallible;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl TryFrom<&str> for TUIdentifier {
    type Error = Infallible;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl Display for TUIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

// TODO: validate the category name is a snaky boy
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct CIdentifier(String);

impl Deref for CIdentifier {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for CIdentifier {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.try_into()
    }
}

impl TryFrom<String> for CIdentifier {
    type Error = Infallible;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(CIdentifier(value))
    }
}

impl TryFrom<&String> for CIdentifier {
    type Error = Infallible;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl TryFrom<&str> for CIdentifier {
    type Error = Infallible;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.to_string().try_into()
    }
}

impl Display for CIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
