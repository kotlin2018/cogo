use std::io::{Read, Write};
use crate::std::io::Closer;

pub mod server;
pub mod client;

pub trait ReadCloser: Read + Closer {}

pub trait WriteCloser: Write + Closer {}

struct NopCloser<R> where R:Read {
   pub reader:R
}

impl <R>Closer for NopCloser<R> {
    fn close(&mut self) -> crate::std::errors::Result<()> {
        Ok(())
    }
}

pub struct Request {
    inner: http::Request<Box<dyn ReadCloser>>,
}

pub struct Response {
    inner: http::Response<Box<dyn WriteCloser>>,
}