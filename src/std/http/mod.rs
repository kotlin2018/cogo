use std::io::{Read, Write};
use crate::std::io::{Closer, ReadCloser, WriteCloser};

pub mod server;
pub mod client;

pub struct Request {
    inner: http::Request<Box<dyn ReadCloser>>,
}

pub struct Response {
    inner: http::Response<Box<dyn WriteCloser>>,
}