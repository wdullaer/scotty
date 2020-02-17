// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use failure::Fail;
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::{env, io};

/// Models a supported shell. Will typically be instantiated from its string representation
///
/// # Examples
/// ```
/// let shell = Shell::from("zsh");
///
/// assert_eq!(shell, Shell::Zsh);
/// ```
#[derive(Debug, PartialEq)]
pub enum Shell {
    Zsh,
    Bash,
}

#[derive(Debug, Fail, PartialEq, Eq)]
pub enum ShellError {
    #[fail(
        display = "`{}` is not a supported shell string representation. Must be one of: [bash, zsh]",
        name
    )]
    UnknownShellName { name: String },
}

impl TryFrom<&str> for Shell {
    type Error = ShellError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().trim() {
            "zsh" => Ok(Shell::Zsh),
            "bash" => Ok(Shell::Bash),
            _ => Err(ShellError::UnknownShellName {
                name: value.to_owned(),
            }),
        }
    }
}

impl TryFrom<&OsStr> for Shell {
    type Error = ShellError;

    fn try_from(value: &OsStr) -> Result<Self, Self::Error> {
        Shell::try_from(value.to_string_lossy().as_ref())
    }
}

const ZSH_INIT: &str = include_str!("scotty.zsh");
const BASH_INIT: &str = include_str!("scotty.bash");

/// Returns the bootstrap script for a specific shell
pub fn init_shell(shell: Shell) -> io::Result<()> {
    let setup_script = match shell {
        Shell::Zsh => ZSH_INIT,
        Shell::Bash => BASH_INIT,
    };

    let scotty_path = env::current_exe()?;
    log::debug!("Detected scotty_path: {}", scotty_path.display());

    print!("{}", interpolate_scotty_path(setup_script, &scotty_path));

    Ok(())
}

// Replace __SCOTTY__ with the path, applying proper escaping
fn interpolate_scotty_path(script: &str, path: &PathBuf) -> String {
    script.replace("__SCOTTY__", &format!("\"{}\"", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_try_from_with_lowercase() {
        let input = "zsh";
        let output = Shell::try_from(input).unwrap();
        let expected = Shell::Zsh;

        assert_eq!(output, expected)
    }

    #[test]
    fn shell_try_from_with_mixed_case() {
        let input = "Zsh";
        let output = Shell::try_from(input).unwrap();
        let expected = Shell::Zsh;

        assert_eq!(output, expected)
    }

    #[test]
    fn shell_try_from_with_whitespace() {
        let input = "zsh ";
        let output = Shell::try_from(input).unwrap();
        let expected = Shell::Zsh;

        assert_eq!(output, expected)
    }

    #[test]
    fn shell_try_from_unknown_shell() {
        let input = "foo";
        let output = Shell::try_from(input);

        assert_eq!(
            output,
            Err(ShellError::UnknownShellName {
                name: input.to_owned()
            })
        )
    }

    #[test]
    fn only_replaces_specific_token() {
        let script = "I am just a normal string";
        let path = PathBuf::new();

        assert_eq!(interpolate_scotty_path(script, &path), script)
    }

    #[test]
    fn should_replace_token_with_value() {
        let script = "__SCOTTY__ init zsh";
        let expected_script = "\"/bin/scotty\" init zsh";
        let path = PathBuf::from("/bin/scotty");

        assert_eq!(interpolate_scotty_path(script, &path), expected_script)
    }

    #[test]
    fn should_replace_token_with_value_whitespace() {
        let script = "__SCOTTY__ init powershell";
        let expected_script = "\"C:\\Program Files\\scotty.exe\" init powershell";
        let path = PathBuf::from("C:\\Program Files\\scotty.exe");

        assert_eq!(interpolate_scotty_path(script, &path), expected_script)
    }

    #[test]
    fn should_replace_in_multiline_script() {
        let script = "echo hello
echo __SCOTTY__";
        let expected_script = "echo hello
echo \"/bin/scotty\"";
        let path = PathBuf::from("/bin/scotty");

        assert_eq!(interpolate_scotty_path(script, &path), expected_script)
    }
}
