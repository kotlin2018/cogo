use std::collections::HashMap;
use crate::std::net::textproto;
use crate::std::errors::Result;

/// Form is a parsed multipart form.
/// Its File parts are stored either in memory or on disk,
/// and are accessible via the *FileHeader's Open method.
/// Its Value parts are stored as strings.
/// Both are keyed by field name.
pub struct Form {
    pub value: HashMap<String, Vec<String>>,
    pub file: HashMap<String, Vec<FileHeader>>,
}

impl Form {
    /// remove_all removes any temporary files associated with a Form.
    pub fn remove_all(&self) -> Result<()> {
        let mut r = Ok(());
        for (k, fhs) in &self.file {
            for fh in fhs {
                if fh.tmpfile.ne("") {
                    if let Err(e) = std::fs::remove_file(fh.tmpfile.as_str()) {
                        r = Err(e.into());
                    }
                }
            }
        }
        r
    }
}

/// A FileHeader describes a file part of a multipart request.
pub struct FileHeader {
    filename: String,
    header: textproto::MIMEHeader,
    size: i64,
    content: Vec<u8>,
    tmpfile: String,
}

impl FileHeader {}

