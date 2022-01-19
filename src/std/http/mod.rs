pub mod jar;
pub mod cookie;
pub mod multipart;

use crate::std::errors::Result;
use std::io::{Read, Write};
use http::{HeaderMap, HeaderValue};
use crate::std::io::{Closer, ReadCloser, WriteCloser};
use crate::std::net::url::Values;
use crate::std::sync::mpmc::Receiver;

pub mod server;
pub mod client;

pub struct Request {
    pub inner: http::Request<Box<dyn ReadCloser>>,
    ///  "HTTP/1.0"
    pub proto: String,
    /// 1
    pub proto_major: i32,
    /// 0
    pub proto_minor: i32,

    ///  get_body defines an optional func to return a new copy of
    ///  Body. It is used for client requests when a redirect requires
    ///  reading the body more than once. Use of GetBody still
    ///  requires setting Body.
    ///
    ///  For server requests, it is unused.
    pub get_body: fn() -> Result<Box<dyn ReadCloser>>,

    ///  content_length records the length of the associated content.
    ///  The value -1 indicates that the length is unknown.
    ///  Values >= 0 indicate that the given number of bytes may
    ///  be read from Body.
    ///
    ///  For client requests, a value of 0 with a non-nil Body is
    ///  also treated as unknown.
    pub content_length: i64,

    ///  transfer_encoding lists the transfer encodings from outermost to
    ///  innermost. An empty list denotes the "identity" encoding.
    ///  transfer_encoding can usually be ignored; chunked encoding is
    ///  automatically added and removed as necessary when sending and
    ///  receiving requests.
    pub transfer_encoding: Vec<String>,

    ///  close indicates whether to close the connection after
    ///  replying to this request (for servers) or after sending this
    ///  request and reading its response (for clients).
    ///
    ///  For server requests, the HTTP server handles this automatically
    ///  and this field is not needed by Handlers.
    ///
    ///  For client requests, setting this field prevents re-use of
    ///  TCP connections between requests to the same hosts, as if
    ///  Transport.DisableKeepAlives were set.
    pub close: bool,
    ///  For server requests, host specifies the host on which the
    ///  URL is sought. For HTTP/1 (per RFC 7230, section 5.4), this
    ///  is either the value of the "host" header or the host name
    ///  given in the URL itself. For HTTP/2, it is the value of the
    ///  ":authority" pseudo-header field.
    ///  It may be of the form "host:port". For international domain
    ///  names, host may be in Punycode or Unicode form. Use
    ///  golang.org/x/net/idna to convert it to either format if
    ///  needed.
    ///  To prevent DNS rebinding attacks, server Handlers should
    ///  validate that the host header has a value for which the
    ///  Handler considers itself authoritative. The included
    ///  ServeMux supports patterns registered to particular host
    ///  names and thus protects its registered Handlers.
    ///
    ///  For client requests, host optionally overrides the host
    ///  header to send. If empty, the request.Write method uses
    ///  the value of URL.host. host may contain an international
    ///  domain name.
    pub host: String,

    ///  form contains the parsed form data, including both the URL
    ///  field's query parameters and the PATCH, POST, or PUT form data.
    ///  This field is only available after ParseForm is called.
    ///  The HTTP client ignores form and uses Body instead.
    pub form: Values,

    ///  post_form contains the parsed form data from PATCH, POST
    ///  or PUT body parameters.
    ///
    ///  This field is only available after ParseForm is called.
    ///  The HTTP client ignores post_form and uses Body instead.
    pub post_form: Values,

    ///  multipart_form is the parsed multipart form, including file uploads.
    ///  This field is only available after ParseMultipartForm is called.
    ///  The HTTP client ignores multipart_form and uses Body instead.
    pub multipart_form: multipart::Form,
    ///  trailer specifies additional headers that are sent after the request
    ///  body.
    //
    ///  For server requests, the trailer map initially contains only the
    ///  trailer keys, with nil values. (The client declares which trailers it
    ///  will later send.)  While the handler is reading from Body, it must
    ///  not reference trailer. After reading from Body returns EOF, trailer
    ///  can be read again and will contain non-nil values, if they were sent
    ///  by the client.
    //
    ///  For client requests, trailer must be initialized to a map containing
    ///  the trailer keys to later send. The values may be nil or their final
    ///  values. The ContentLength must be 0 or -1, to send a chunked request.
    ///  After the HTTP request is sent the map values can be updated while
    ///  the request body is read. Once the body returns EOF, the caller must
    ///  not mutate trailer.
    ///
    ///  Few HTTP clients, servers, or proxies support HTTP trailers.
    pub trailer: HeaderMap<HeaderValue>,

    ///  remote_addr allows HTTP servers and other software to record
    ///  the network address that sent the request, usually for
    ///  logging. This field is not filled in by ReadRequest and
    ///  has no defined format. The HTTP server in this package
    ///  sets remote_addr to an "IP:port" address before invoking a
    ///  handler.
    ///  This field is ignored by the HTTP client.
    pub remote_addr: String,

    ///  request_uri is the unmodified request-target of the
    ///  request-Line (RFC 7230, Section 3.1.1) as sent by the client
    ///  to a server. Usually the URL field should be used instead.
    ///  It is an error to set this field in an HTTP client request.
    pub request_uri: String,

    ///  cancel is an optional channel whose closure indicates that the client
    ///  request should be regarded as canceled. Not all implementations of
    ///  RoundTripper may support cancel.
    ///
    ///  For server requests, this field is not applicable.
    ///
    ///  Deprecated: Set the request's context with NewRequestWithContext
    ///  instead. If a request's cancel field and context are both
    ///  set, it is undefined whether cancel is respected.
    pub cancel: Receiver<()>,

    ///  response is the redirect response which caused this request
    ///  to be created. This field is only populated during client
    ///  redirects.
    pub response: Box<Response>,
}

pub struct Response {
    pub inner: http::Response<Box<dyn WriteCloser>>,
    pub status: String,
    /// e.g. "200 OK"
    pub status_code: i32,
    /// e.g. 200
    pub proto: String,
    /// e.g. "HTTP/1.0"
    pub proto_major: i32,
    /// e.g. 1
    pub proto_minor: i32,
    /// e.g. 0
    ///  content_length records the length of the associated content.
    ///  The value -1 indicates that the length is unknown.
    ///  Values >= 0 indicate that the given number of bytes may
    ///  be read from Body.
    ///
    ///  For client requests, a value of 0 with a non-nil Body is
    ///  also treated as unknown.
    pub content_length: i64,
    ///  Contains transfer encodings from outer-most to inner-most. Value is
    ///  nil, means that "identity" encoding is used.
    pub transfer_encoding: Vec<String>,
    ///  close indicates whether to close the connection after
    ///  replying to this request (for servers) or after sending this
    ///  request and reading its response (for clients).
    ///
    ///  For server requests, the HTTP server handles this automatically
    ///  and this field is not needed by Handlers.
    ///
    ///  For client requests, setting this field prevents re-use of
    ///  TCP connections between requests to the same hosts, as if
    ///  Transport.DisableKeepAlives were set.
    pub close: bool,
    ///  uncompressed reports whether the response was sent compressed but
    ///  was decompressed by the http package. When true, reading from
    ///  Body yields the uncompressed content instead of the compressed
    ///  content actually set from the server, ContentLength is set to -1,
    ///  and the "Content-Length" and "Content-Encoding" fields are deleted
    ///  from the responseHeader. To get the original response from
    ///  the server, set Transport.DisableCompression to true.
    pub uncompressed: bool,
    ///  trailer specifies additional headers that are sent after the request
    ///  body.
    //
    ///  For server requests, the trailer map initially contains only the
    ///  trailer keys, with nil values. (The client declares which trailers it
    ///  will later send.)  While the handler is reading from Body, it must
    ///  not reference trailer. After reading from Body returns EOF, trailer
    ///  can be read again and will contain non-nil values, if they were sent
    ///  by the client.
    //
    ///  For client requests, trailer must be initialized to a map containing
    ///  the trailer keys to later send. The values may be nil or their final
    ///  values. The ContentLength must be 0 or -1, to send a chunked request.
    ///  After the HTTP request is sent the map values can be updated while
    ///  the request body is read. Once the body returns EOF, the caller must
    ///  not mutate trailer.
    ///
    ///  Few HTTP clients, servers, or proxies support HTTP trailers.
    pub trailer: HeaderMap<HeaderValue>,

    ///  request is the request that was sent to obtain this Response.
    ///  request's Body is nil (having already been consumed).
    ///  This is only populated for Client requests.
    pub request: Box<Request>,
}