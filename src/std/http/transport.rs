use crate::std::http::client::RoundTripper;
use crate::std::http::{Request, Response};

///Transport is an implementation of RoundTripper that supports HTTP,
///HTTPS, and HTTP proxies (for either HTTP or HTTPS with CONNECT).
///
///By default, Transport caches connections for future re-use.
///This may leave many open connections when accessing many hosts.
///This behavior can be managed using Transport's CloseIdleConnections method
///and the MaxIdleConnsPerHost and DisableKeepAlives fields.
///
///Transports should be reused instead of created as needed.
///Transports are safe for concurrent use by multiple goroutines.
///
///A Transport is a low-level primitive for making HTTP and HTTPS requests.
///For high-level functionality, such as cookies and redirects, see Client.
///
///Transport uses HTTP/1.1 for HTTP URLs and either HTTP/1.1 or HTTP/2
///for HTTPS URLs, depending on whether the server supports HTTP/2,
///and how the Transport is configured. The DefaultTransport supports HTTP/2.
///To explicitly enable HTTP/2 on a transport, use golang.org/x/net/http2
///and call ConfigureTransport. See the package docs for more about HTTP/2.
///
///Responses with status codes in the 1xx range are either handled
///automatically (100 expect-continue) or ignored. The one
///exception is HTTP status code 101 (Switching Protocols), which is
///considered a terminal status and returned by RoundTrip. To see the
///ignored 1xx responses, use the httptrace trace package's
///ClientTrace.Got1xxResponse.
///
///Transport only retries a request upon encountering a network error
///if the request is idempotent and either has no body or has its
///Request.GetBody defined. HTTP requests are considered idempotent if
///they have HTTP methods GET, HEAD, OPTIONS, or TRACE; or if their
///Header map contains an "Idempotency-Key" or "X-Idempotency-Key"
///entry. If the idempotency key value is a zero-length slice, the
///request is treated as idempotent but the header is not sent on the
///wire.
pub struct Transport{
    
}

impl RoundTripper for Transport{
    fn roundtrip(&self, req: Request) -> crate::std::errors::Result<Response> {
        todo!()
    }
}