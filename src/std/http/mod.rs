pub mod server;
pub mod client;

pub struct Request {
    inner: http::Request<Box<dyn std::io::Read>>,
}

pub struct Response {
    inner: http::Response<Box<dyn std::io::Write>>,
}