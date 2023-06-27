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

impl PathNode {
    pub fn from_path(path: &Path) -> Result<PathNode, std::io::Error> {
        if path.is_dir() {
            std::fs::read_dir(path)?
                .filter_map(Result::ok)
                .map(|entry| {
                    let path = entry.path();
                    let file_name = path.file_name().expect("got file but has no dir name");
                    let file_name = file_name.to_str().ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!("Could not convert dir name to UTF-8: {file_name:?}"),
                        )
                    })?;
                    PathNode::from_path(&path).map(|x| (file_name.to_string(), x))
                })
                .collect::<Result<BTreeMap<_, _>, _>>()
                .map(PathNode::Directory)
        } else if path.is_file() {
            // let file_name = path.file_name().expect("got file but has no file name");
            // let file_name = file_name.to_str().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Could not convert file name to UTF-8: {file_name:?}")))?;
            Ok(PathNode::File(
                std::fs::read_to_string(path).unwrap().into_bytes(),
            ))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File or directory not found",
            ))
        }
    }

    pub fn into_directory(self) -> Option<BTreeMap<String, PathNode>> {
        match self {
            PathNode::Directory(d) => Some(d),
            PathNode::File(_) => None,
        }
    }

    pub fn into_file(self) -> Option<Vec<u8>> {
        match self {
            PathNode::Directory(_) => None,
            PathNode::File(f) => Some(f),
        }
    }

    pub fn write(self, prefix: &Path) -> std::io::Result<()> {
        match self {
            PathNode::File(data) => {
                std::fs::write(prefix, data)?;
                Ok(())
            }
            PathNode::Directory(tree) => {
                std::fs::create_dir_all(prefix)?;
                write_directory(prefix, tree)
            }
        }
    }
}

fn write_directory(prefix: &Path, tree: BTreeMap<String, PathNode>) -> std::io::Result<()> {
    for (k, v) in tree.into_iter() {
        let path = prefix.join(&k);
        match v {
            PathNode::File(data) => {
                std::fs::write(path, data)?;
            }
            PathNode::Directory(tree) => {
                std::fs::create_dir_all(&path)?;
                write_directory(&prefix.join(&k), tree)?;
            }
        }
    }

    Ok(())
}

#[derive(Clone)]
pub struct BTreeKeyedSet<K, V: Keyed<K>> {
    map: BTreeMap<K, V>,
}

impl<K, V: Keyed<K>> Default for BTreeKeyedSet<K, V> {
    fn default() -> Self {
        Self {
            map: Default::default(),
        }
    }
}

impl<K, V> Debug for BTreeKeyedSet<K, V>
where
    K: Debug,
    V: Debug + Keyed<K>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.map.fmt(f)
    }
}

impl<K, V> BTreeKeyedSet<K, V>
where
    V: Keyed<K>,
{
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    pub fn into_iter(self) -> impl Iterator<Item = (K, V)> {
        self.map.into_iter()
    }
}

impl<K, V> BTreeKeyedSet<K, V>
where
    K: Ord,
    V: Keyed<K>,
{
    pub fn insert(&mut self, value: V) -> Option<V> {
        self.map.insert(value.key(), value)
    }

    pub fn from_set(value: BTreeSet<V>, keyer: fn(&V) -> K) -> Self {
        let map = value
            .into_iter()
            .map(|v| (keyer(&v), v))
            .collect::<BTreeMap<_, _>>();
        Self { map }
    }
}

impl<K, V> Deref for BTreeKeyedSet<K, V>
where
    V: Keyed<K>,
{
    type Target = BTreeMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<K, V> DerefMut for BTreeKeyedSet<K, V>
where
    V: Keyed<K>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

pub trait Keyed<V> {
    fn key(&self) -> V;
}
