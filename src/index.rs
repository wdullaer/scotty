extern crate failure;
extern crate fst;
extern crate sled;

use std::io;
use std::path::PathBuf;
use std::ops::Deref;
use std::convert::TryInto;
use std::time::SystemTime;

use failure::Error;
use fst::{Set, SetBuilder};
use sled::{Db, Tree};

const PATHS_TREE: &str = "paths";
const MAIN_TREE: &str = "main";
const INDEX_KEY: &str = "index";

pub struct Index {
    main: Tree,
    paths: Tree,
}

impl Index {
    // TODO: make this take a config object
    pub fn new() -> Result<Index, sled::Error> {
        // TODO: retrieve the path from config
        let db = Db::open("scotty.db")?;
        let main_tree = db.open_tree(MAIN_TREE)?;
        let paths_tree = db.open_tree(PATHS_TREE)?;
        Ok(Index{
            main: main_tree,
            paths: paths_tree,
        })
    }

    /// add adds a path to the database and update the indexes
    pub fn add(&self, path_buf: &PathBuf) -> Result<(), Error> {
        if !path_buf.exists() {
            return Err(Error::from(io::Error::new(io::ErrorKind::NotFound, "Path does not exist")))
        }

        // Check if the path is already known and update its last modified timestamp
        let path_string = path_buf.to_string_lossy();
        let path_bytes = path_string.as_bytes();
        // TODO: see if we can use Instant and if bincode is the best way to handle this
        let time_bytes = bincode::serialize(&SystemTime::now())?;
        match self.paths.insert(path_bytes, time_bytes)? {
            // New path: update the fst
            None => self.update_paths_index(path_bytes),
            _ => Ok(()),
        }
    }

    // update_paths_index update the fts index with the new path
    fn update_paths_index(&self, path_bytes: &[u8]) -> Result<(), Error> {
        let delta_fst = Set::from_iter(vec![path_bytes])?;

        let paths_fst = match self.main.get(INDEX_KEY)? {
            Some(bytes) => Set::from_bytes(bytes.deref().try_into()?)?,
            None => Set::default(),
        };

        let new_fst = merge_fst_sets(&delta_fst, &paths_fst)?;

        self.main.insert(INDEX_KEY, new_fst.as_fst().as_bytes())?;
        Ok(())
    }
}

/// merge_fst_sets merges (creates a union) between two fst::Set and returns the result a newly allocated fst::Set
fn merge_fst_sets(delta_set: &Set, paths_set: &Set) -> fst::Result<Set> {
        let stream = paths_set.op()
            .add(delta_set.stream())
            .union();
        
        let mut paths_builder = SetBuilder::memory();
        paths_builder.extend_stream(stream)?;
        paths_builder
            .into_inner()
            .and_then(Set::from_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_fst_sets_both_empty() {
        let fst1 = Set::default();
        let fst2 = Set::default();

        let result = merge_fst_sets(&fst1, &fst2).unwrap();

        assert!(result.is_empty())
    }

    #[test]
    fn merge_fst_sets_one_empty() {
        let fst1 = Set::default();
        let fst2 = Set::from_iter(vec!["bar", "foo"]).unwrap();

        let result = merge_fst_sets(&fst1, &fst2).unwrap();

        assert_eq!(result.len(), fst2.len());
        assert!(result.is_subset(fst2.stream()))
    }
    
    #[test]
    fn merge_fst_sets_no_empty() {
        let fst1 = Set::from_iter(vec!["abc", "def"]).unwrap();
        let fst2 = Set::from_iter(vec!["bar", "foo"]).unwrap();

        let result = merge_fst_sets(&fst1, &fst2).unwrap();

        assert_eq!(result.stream().into_strs().unwrap(), vec!["abc", "bar", "def", "foo"])
    }
}