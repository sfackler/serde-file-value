use std::{io, path::Path};

pub use de::Deserializer;

mod de;

pub trait Listen {
    fn on_read(&mut self, path: &Path, contents: &[u8]);

    fn on_error(&mut self, path: &Path, error: &io::Error);
}

pub struct NopListener;

impl Listen for NopListener {
    #[inline]
    fn on_read(&mut self, _: &Path, _: &[u8]) {}

    #[inline]
    fn on_error(&mut self, _: &Path, _: &io::Error) {}
}

#[cfg(test)]
mod test {
    use std::{
        collections::{HashMap, HashSet},
        fs,
        path::PathBuf,
    };

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

    #[derive(Debug, PartialEq)]
    struct TestListener {
        files: HashMap<PathBuf, Vec<u8>>,
        errors: HashSet<PathBuf>,
    }

    impl Listen for TestListener {
        fn on_read(&mut self, path: &Path, contents: &[u8]) {
            self.files.insert(path.to_owned(), contents.to_owned());
        }

        fn on_error(&mut self, path: &Path, _: &io::Error) {
            self.errors.insert(path.to_owned());
        }
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

        let mut listener = TestListener {
            files: HashMap::new(),
            errors: HashSet::new(),
        };
        let mut deserializer = serde_json::Deserializer::from_str(&config);
        let deserializer = Deserializer::new(&mut deserializer, &mut listener);

        let config = Config::deserialize(deserializer).unwrap();

        let expected = Config {
            sub: Subconfig {
                file: vec!["hunter2".to_string()],
                inline: "${foobar}".to_string(),
            },
        };

        assert_eq!(config, expected);

        let expected = TestListener {
            files: HashMap::from([(file.path().to_owned(), "hunter2".as_bytes().to_vec())]),
            errors: HashSet::new(),
        };
        assert_eq!(listener, expected);
    }

    #[test]
    fn io_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("bogus");

        let config = format!("\"${{file:{}}}\"", file.display());

        let mut listener = TestListener {
            files: HashMap::new(),
            errors: HashSet::new(),
        };
        let mut deserializer = serde_json::Deserializer::from_str(&config);
        let deserializer = Deserializer::new(&mut deserializer, &mut listener);

        String::deserialize(deserializer).unwrap_err();

        let expected = TestListener {
            files: HashMap::new(),
            errors: HashSet::from([file]),
        };
        assert_eq!(listener, expected);
    }
}
