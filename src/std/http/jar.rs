use crate::std::http::cookie::Cookie;

// A CookieJar manages storage and use of cookies in HTTP requests.
//
// Implementations of CookieJar must be safe for concurrent use by multiple
// goroutines.
//
// The net/http/cookiejar package provides a CookieJar implementation.
pub trait CookieJar {
    // SetCookies handles the receipt of the cookies in a reply for the
    // given URL.  It may or may not choose to save the cookies, depending
    // on the jar's policy and implementation.
    fn set_cookies(&mut self, u: http::Uri, cookies: Vec<Cookie>);
    // Cookies returns the cookies to send in a request for the given URL.
    // It is up to the implementation to honor the standard cookie use
    // restrictions such as in RFC 6265.
    fn cookies(&self, u: http::Uri) -> Vec<Cookie>;
}