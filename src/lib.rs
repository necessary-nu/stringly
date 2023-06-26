use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    ops::{Deref, DerefMut},
    path::Path,
};

pub mod flt;
pub mod ir;
pub mod translate;
pub mod ts;
pub mod xlsx;

pub enum PathNode {
    File(Vec<u8>),
    Directory(BTreeMap<String, PathNode>),
}

pub fn write_path_tree(prefix: &Path, tree: BTreeMap<String, PathNode>) -> std::io::Result<()> {
    for (k, v) in tree.into_iter() {
        let path = prefix.join(&k);
        match v {
            PathNode::File(data) => {
                std::fs::write(path, data)?;
            }
            PathNode::Directory(tree) => {
                std::fs::create_dir_all(&path)?;
                write_path_tree(&prefix.join(&k), tree)?;
            }
        }
    }

    Ok(())
}

#[derive(Clone)]
pub struct BTreeKeyedSet<K, V> {
    map: BTreeMap<K, V>,
    keyer: fn(&V) -> K,
}

impl<K: Debug, V: Debug> Debug for BTreeKeyedSet<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.map.fmt(f)
    }
}

impl<K, V> BTreeKeyedSet<K, V> {
    pub fn new(keyer: fn(&V) -> K) -> Self {
        Self {
            map: Default::default(),
            keyer,
        }
    }

    pub fn into_iter(self) -> impl Iterator<Item = (K, V)> {
        self.map.into_iter()
    }
}

impl<K: Ord, V> BTreeKeyedSet<K, V> {
    pub fn insert(&mut self, value: V) {
        self.map.insert((self.keyer)(&value), value);
    }

    pub fn from_set(value: BTreeSet<V>, keyer: fn(&V) -> K) -> Self {
        let map = value
            .into_iter()
            .map(|v| (keyer(&v), v))
            .collect::<BTreeMap<_, _>>();
        Self { map, keyer }
    }
}

impl<K, V> Deref for BTreeKeyedSet<K, V> {
    type Target = BTreeMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<K, V> DerefMut for BTreeKeyedSet<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}
