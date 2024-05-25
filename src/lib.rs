//! A Serde deserializer which transparently loads files as string values.
//!
//! # Usage
//!
//! Assume we have a `/mnt/secrets/my_secret` file that looks like:
//!
//! ```text
//! hunter2
//! ```
//!
//! And a `conf/config.json` that looks like:
//!
//! ```json
//! {
//!     "secret_value": "${file:/mnt/secrets/my_secret}"
//! }
//! ```
//! ```no_run
//! use std::{fs, io, path::Path};
//!
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct Config {
//!     secret_value: String,
//! }
//!
//! let config = fs::read("conf/config.json").unwrap();
//!
//! let mut deserializer = serde_json::Deserializer::from_slice(&config);
//! let mut callback = |_: &Path, _: &io::Result<Vec<u8>>| {};
//! let deserializer = serde_file_value::Deserializer::new(&mut deserializer, &mut callback);
//! let config = Config::deserialize(deserializer).unwrap();
//!
//! assert_eq!(config.secret_value, "hunter2");
//! ```
#![warn(missing_docs)]
pub use de::Deserializer;

mod de;

#[cfg(test)]
mod test {
    use std::{fs, io, path::Path};

    use serde::Deserialize;
    use tempfile::NamedTempFile;

    use super::*;

    #[derive(Deserialize, PartialEq, Debug)]
    struct Config {
        sub: Subconfig,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct Subconfig {
        file: Vec<String>,
        inline: String,
    }

    #[test]
    fn smoke() {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), "hunter2").unwrap();

        let config = format!(
            r#"
{{
    "sub": {{
        "file": [
            "${{file:{}}}"
        ],
        "inline": "${{foobar}}"
    }}
}}
        "#,
            file.path().display(),
        );

        let mut deserializer = serde_json::Deserializer::from_str(&config);
        let mut files = vec![];
        let mut cb = |path: &Path, r: &io::Result<Vec<u8>>| {
            files.push((path.to_owned(), r.as_ref().ok().cloned()))
        };
        let deserializer = Deserializer::new(&mut deserializer, &mut cb);

        let config = Config::deserialize(deserializer).unwrap();

        let expected = Config {
            sub: Subconfig {
                file: vec!["hunter2".to_string()],
                inline: "${foobar}".to_string(),
            },
        };

        assert_eq!(config, expected);

        let expected = vec![(file.path().to_owned(), Some("hunter2".as_bytes().to_vec()))];
        assert_eq!(files, expected);
    }

    #[test]
    fn io_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("bogus");

        let config = format!("\"${{file:{}}}\"", file.display());

        let mut deserializer = serde_json::Deserializer::from_str(&config);
        let mut files = vec![];
        let mut cb = |path: &Path, r: &io::Result<Vec<u8>>| {
            files.push((path.to_owned(), r.as_ref().ok().cloned()))
        };
        let deserializer = Deserializer::new(&mut deserializer, &mut cb);

        String::deserialize(deserializer).unwrap_err();

        let expected = vec![(file.to_path_buf(), None)];
        assert_eq!(files, expected);
    }
}
