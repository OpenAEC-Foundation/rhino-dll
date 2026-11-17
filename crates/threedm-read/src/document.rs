//! A Rhino file read into memory with its model: the model borrows the point arrays of the file
//! (point clouds are read only when used), so the two are kept together.

use crate::{read_with, File, Options, R};

/// An open Rhino file: its bytes and the model read from them.
pub struct Document {
    // `file` borrows from `data`: it is dropped first (see `Drop`), and `data` never moves (it
    // is a heap allocation owned through a raw pointer).
    file: std::mem::ManuallyDrop<File<'static>>,
    data: *mut [u8],
}

impl Document {
    /// Reads the file at `path`.
    pub fn open(path: impl AsRef<std::path::Path>) -> R<Document> {
        let data = std::fs::read(path.as_ref()).map_err(|e| format!("{}: {e}", path.as_ref().display()))?;
        Document::from_bytes(data, &Options::default())
    }

    /// Reads a file from its bytes.
    pub fn from_bytes(data: Vec<u8>, options: &Options) -> R<Document> {
        let data: *mut [u8] = Box::into_raw(data.into_boxed_slice());
        // SAFETY: `data` stays allocated, unmoved and unchanged until `file` is dropped.
        let bytes: &'static [u8] = unsafe { &*data };
        match read_with(bytes, options) {
            Ok(file) => Ok(Document { file: std::mem::ManuallyDrop::new(file), data }),
            Err(e) => {
                // SAFETY: nothing borrows `data` any more.
                drop(unsafe { Box::from_raw(data) });
                Err(e)
            }
        }
    }

    /// The model.
    pub fn file(&self) -> &File<'_> {
        &self.file
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        // SAFETY: the model goes first, then the bytes it borrowed; neither is used again.
        unsafe {
            std::mem::ManuallyDrop::drop(&mut self.file);
            drop(Box::from_raw(self.data));
        }
    }
}

// SAFETY: the document owns its bytes and only hands out shared references to them.
unsafe impl Send for Document {}
unsafe impl Sync for Document {}
