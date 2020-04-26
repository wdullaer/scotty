// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use chrono::NaiveDateTime;
use failure::Error;
use std::convert::TryInto;
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::index::PathIndexEntry;

// Prints a slice of PathBufs in a single line seperated by a space
pub fn print_path_slice(paths: &[PathBuf]) -> Result<(), Error> {
    let stdout = io::stdout();
    let std_lock = stdout.lock();
    let mut handle = io::BufWriter::new(std_lock);

    for item in paths {
        writeln!(handle, "{}", item.display())?;
    }
    Ok(())
}

// Prints the Vec of index entries as line delimited json objects on stdout
pub fn print_json(index_entries: &[PathIndexEntry]) -> Result<(), Error> {
    let stdout = io::stdout();
    let std_lock = stdout.lock();
    let mut handle = io::BufWriter::new(std_lock);

    for entry in index_entries {
        serde_json::to_writer(&mut handle, entry)?;
        writeln!(handle)?;
    }
    Ok(())
}

// Prints the Vec of index entries as a human readable table on stdout
pub fn print_human(index_entries: &[PathIndexEntry]) -> Result<(), Error> {
    let stdout = io::stdout();
    let std_lock = stdout.lock();
    let handle = io::BufWriter::new(std_lock);
    let mut tab_handle = tabwriter::TabWriter::new(handle);

    writeln!(tab_handle, "PATH\tTIMESTAMP")?;

    for entry in index_entries {
        writeln!(
            tab_handle,
            "{}\t{}",
            entry.path.display(),
            get_datetime_string(&entry.timestamp)
        )?;
    }

    Ok(tab_handle.flush()?)
}

// Converts a systemtime into a human readable string
// Panics if the time is before the UNIX_EPOCH or if the number of seconds after the epoch does not fit in a int64
// (This should be about 292471208677 years, so I'm ok to run with that assumption)
fn get_datetime_string(systime: &SystemTime) -> String {
    let duration = systime
        .duration_since(UNIX_EPOCH)
        .expect("timestamp should be after UNIX_EPOCH");
    let datetime = NaiveDateTime::from_timestamp(
        duration.as_secs().try_into().unwrap(),
        duration.subsec_nanos(),
    );
    format!("{}", datetime)
}
