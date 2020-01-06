// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use directories::ProjectDirs;

use crate::index::IndexError;

/// Constructs a sled config that will write the db to
/// the data directory for this application based on host OS standards
/// It will return an error if no data directory can be determined (we
/// might make this location configurable in the future)
pub fn get_index_config() -> Result<sled::Config, IndexError> {
    let mut db_path = ProjectDirs::from("com", "wdullaer", "scotty")
        .ok_or(IndexError::BadDataDirectoryError)?
        .data_dir()
        .to_path_buf();
    db_path.push("scotty.db");
    Ok(sled::Config::new().path(db_path.as_path()))
}
