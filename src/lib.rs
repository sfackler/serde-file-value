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
