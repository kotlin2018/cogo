use std::collections::HashMap;
use crate::std::net::textproto;

// Form is a parsed multipart form.
// Its File parts are stored either in memory or on disk,
// and are accessible via the *FileHeader's Open method.
// Its Value parts are stored as strings.
// Both are keyed by field name.
pub struct Form{
    pub value:HashMap<String,Vec<String>>,
    pub file:HashMap<String,Vec<FileHeader>>
}

// A FileHeader describes a file part of a multipart request.
pub struct FileHeader{
    Filename:String,
    Header:textproto::MIMEHeader,
    Size:i64,
    content: Vec<u8>,
    tmpfile:String,
}