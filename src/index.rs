extern crate failure;
extern crate fst;
extern crate sled;

use std::convert::TryInto;
use std::io;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use failure::Error;
use fst::automaton::Subsequence;
use fst::{IntoStreamer, Set, SetBuilder};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
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
        Ok(Index {
            main: main_tree,
            paths: paths_tree,
        })
    }

    /// add adds a path to the database and update the indexes
    pub fn add(&self, path_buf: &PathBuf) -> Result<(), Error> {
        if !path_buf.exists() {
            return Err(Error::from(io::Error::new(
                io::ErrorKind::NotFound,
                "Path does not exist",
            )));
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

    pub fn search(&self, target: &str) -> Result<Option<PathBuf>, Error> {
        // Get the index from the database
        let fst_index = match self.main.get(INDEX_KEY)? {
            Some(bytes) => Set::from_bytes(bytes.deref().try_into()?)?,
            None => Set::default(),
        };

        // Create the query automaton and run it
        let subseq = Subsequence::new(target);
        let stream = fst_index.search(subseq).into_stream();
        let results = stream.into_strs()?;

        // Score the results
        let score_vec = score_results(&results, target);

        let best_score = self.get_best_score(score_vec)?;

        Ok(best_score.map(|p| p.path.clone()))
    }

    fn get_timestamp(&self, path: &Path) -> Result<Option<SystemTime>, Error> {
        let time_bytes = &self.paths.get(path.to_string_lossy().as_bytes())?;
        match time_bytes {
            None => Ok(None),
            Some(b) => Ok(bincode::deserialize(b)?),
        }
    }

    // get_best_score consumes the vector and returns the item with the best score
    // It will use the timestamp stored in the database as a tie-breaker
    // Care is taken to minimize the amount of database lookups
    fn get_best_score(&self, mut results: Vec<Score>) -> Result<Option<Score>, Error> {
        if results.is_empty() {
            return Ok(None);
        }

        // Get max score
        results.sort();
        let max_score = results.last().unwrap().score;

        // Get timestamp for ties
        results.retain(|x| x.score == max_score);
        for score in results.iter_mut() {
            score.timestamp = self.get_timestamp(&score.path)?;
        }
        results.sort();

        // Return best result
        Ok(results.pop())
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

/// score_results computes the fuzzy matching score of each result against the target string
fn score_results(results: &[String], target: &str) -> Vec<Score> {
    let scorer = SkimMatcherV2::default();
    results
        .iter()
        .map(|item| Score {
            path: PathBuf::from(item),
            score: scorer.fuzzy_match(item, target).unwrap_or_default(),
            timestamp: None,
        })
        .collect::<Vec<_>>()
}

/// merge_fst_sets merges (creates a union) between two fst::Set and returns the result a newly allocated fst::Set
fn merge_fst_sets(delta_set: &Set, paths_set: &Set) -> fst::Result<Set> {
    let stream = paths_set.op().add(delta_set.stream()).union();

    let mut paths_builder = SetBuilder::memory();
    paths_builder.extend_stream(stream)?;
    paths_builder.into_inner().and_then(Set::from_bytes)
}

#[derive(Ord, PartialOrd, PartialEq, Eq)]
struct Score {
    score: i64,
    timestamp: Option<SystemTime>,
    path: PathBuf,
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

        assert_eq!(
            result.stream().into_strs().unwrap(),
            vec!["abc", "bar", "def", "foo"]
        )
    }

    #[test]
    fn score_result_equal_length() {
        let input = vec!["foo".to_owned(), "bar".to_owned()];
        let pattern = "abc";

        let result = score_results(input.as_slice(), pattern);

        assert_eq!(result.len(), input.len())
    }

    #[test]
    fn score_result_empty_input() {
        let input = Vec::<String>::new();
        let pattern = "abc";

        let result = score_results(input.as_slice(), pattern);

        assert!(result.is_empty())
    }
}
