use std::{collections::BTreeMap, path::Path};

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
