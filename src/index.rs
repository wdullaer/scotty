// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::convert::TryInto;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use failure::{Error, Fail};
use fst::automaton::Subsequence;
use fst::{IntoStreamer, Set, SetBuilder};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use sled::{Config, Tree};

const PATHS_TREE: &str = "paths";
const MAIN_TREE: &str = "main";
const INDEX_KEY: &str = "index";

#[derive(Debug, Fail, PartialEq, Eq)]
pub enum IndexError {
    #[fail(display = "No path found for pattern `{}`", pattern)]
    NoResultsError { pattern: String },
    #[fail(display = "Path `{}` does not exist", path)]
    PathDoesNotExistError { path: String },
    #[fail(display = "Path `{}` is not absolute", path)]
    RelativePathError { path: String },
    #[fail(display = "Could determine writable location for index data")]
    BadDataDirectoryError,
}

pub struct Index {
    main: Tree,
    paths: Tree,
}

impl Index {
    /// open opens and configures a new sled database based on the provided
    /// config
    pub fn open(config: Config) -> Result<Index, sled::Error> {
        log::debug!("Opening db for config: {:?}", config);
        let db = config.open()?;
        let main_tree = db.open_tree(MAIN_TREE)?;
        let paths_tree = db.open_tree(PATHS_TREE)?;
        Ok(Index {
            main: main_tree,
            paths: paths_tree,
        })
    }

    /// add adds a path to the database and update the indexes
    pub fn add(&self, path_buf: &PathBuf) -> Result<(), Error> {
        log::debug!("Adding path to index: {}", path_buf.display());
        if !path_buf.is_dir() {
            return Err(Error::from(IndexError::PathDoesNotExistError {
                path: path_buf.to_string_lossy().into_owned(),
            }));
        }
        if !path_buf.is_absolute() {
            return Err(Error::from(IndexError::RelativePathError {
                path: path_buf.to_string_lossy().into_owned(),
            }));
        }

        // Check if the path is already known and update its last modified timestamp
        let path_string = path_buf.to_string_lossy();
        let path_bytes = path_string.as_bytes();

        let time_bytes = bincode::serialize(&SystemTime::now())?;
        match self.paths.insert(path_bytes, time_bytes)? {
            // New path: update the fst
            None => self.update_paths_index(path_bytes, merge_fst_sets),
            _ => Ok(()),
        }
    }

    pub fn search(&self, target: &str) -> Result<Option<PathBuf>, Error> {
        log::debug!("Searching target in index: {}", target);
        // Get the index from the database
        let fst_index = match self.main.get(INDEX_KEY)? {
            Some(bytes) => Set::from_bytes(bytes.deref().try_into()?)?,
            None => Set::default(),
        };

        // Create the query automaton and run it
        let subseq = Subsequence::new(target);
        let stream = fst_index.search(subseq).into_stream();
        let results = stream.into_strs()?;
        log::trace!("FST result set: {:?}", results);

        // Score the results
        let score_vec = score_results(&results, target);
        log::trace!("Scored FST result set: {:?}", score_vec);

        let best_score = self.get_best_score(score_vec)?;
        log::trace!("Best result: {:?}", best_score);

        Ok(best_score.map(|p| p.path.clone()))
    }

    pub fn delete(&self, path_buf: &PathBuf) -> Result<(), Error> {
        log::debug!("Deleting path from index: {}", path_buf.display());
        let path_string = path_buf.to_string_lossy();
        let path_bytes = path_string.as_bytes();
        match self.paths.remove(path_bytes)? {
            None => Ok(()),
            Some(_) => self.update_paths_index(path_bytes, remove_fst_set),
        }
    }

    fn get_timestamp(&self, path: &Path) -> Result<Option<SystemTime>, Error> {
        let time_bytes = self.paths.get(path.to_string_lossy().as_bytes())?;
        Ok(time_bytes
            .map(|x| bincode::deserialize::<SystemTime>(x.as_ref()))
            .transpose()?)
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

        results.retain(|x| x.score == max_score);
        // Get timestamp for ties
        if results.len() > 1 {
            for score in results.iter_mut() {
                score.timestamp = self.get_timestamp(&score.path)?;
            }
            results.sort();
        }

        // Return best result
        Ok(results.pop())
    }

    // update_paths_index updates the fts index with the new path using the passed in operation (merge or remove)
    fn update_paths_index<F>(&self, path_bytes: &[u8], op: F) -> Result<(), Error>
    where
        F: Fn(&Set, &Set) -> fst::Result<Set>,
    {
        log::debug!(
            "Updating path index: {}",
            std::str::from_utf8(path_bytes).unwrap_or_default()
        );
        let delta_fst = Set::from_iter(vec![path_bytes])?;

        let paths_fst = match self.main.get(INDEX_KEY)? {
            Some(bytes) => Set::from_bytes(bytes.deref().try_into()?)?,
            None => Set::default(),
        };

        let new_fst = op(&delta_fst, &paths_fst)?;

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
    log::debug!("Merging fst set");
    let stream = paths_set.op().add(delta_set.stream()).union();

    let mut paths_builder = SetBuilder::memory();
    paths_builder.extend_stream(stream)?;
    paths_builder.into_inner().and_then(Set::from_bytes)
}

fn remove_fst_set(delta_set: &Set, paths_set: &Set) -> fst::Result<Set> {
    log::debug!("Removing fst set");
    let stream = paths_set.op().add(delta_set.stream()).difference();

    let mut paths_builder = SetBuilder::memory();
    paths_builder.extend_stream(stream)?;
    paths_builder.into_inner().and_then(Set::from_bytes)
}

#[derive(Ord, PartialOrd, PartialEq, Eq, Debug)]
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
