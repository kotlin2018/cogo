use std::ops::Index;
use http::HeaderValue;

pub struct Cookie {
    name: String,
    value: String,
    path: String,
    domain: String,
    expires: String,
    raw_expires: String,
    // for reading cookies only
    // MaxAge=0 means no 'Max-Age' attribute specified.
    // MaxAge<0 means delete cookie now, equivalently 'Max-Age: 0'
    // MaxAge>0 means Max-Age attribute present and given in seconds
    max_age: i32,
    secure: bool,
    http_only: bool,
    same_site: SameSite,
    raw: String,
    unparsed: Vec<String>, // Raw text of unparsed attribute-value pairs
}

// SameSite allows a server to define a cookie attribute making it impossible for
// the browser to send this cookie along with cross-site requests. The main
// goal is to mitigate the risk of cross-origin information leakage, and provide
// some protection against cross-site request forgery attacks.
//
// See https://tools.ietf.org/html/draft-ietf-httpbis-cookie-same-site-00 for details.
pub type SameSite = i32;

pub const SameSiteDefaultMode: SameSite = 1;
pub const SameSiteLaxMode: SameSite = 1;
pub const SameSiteStrictMode: SameSite = 1;
pub const SameSiteNoneMode: SameSite = 1;

// readSetCookies parses all "Set-Cookie" values from
// the header h and returns the successfully parsed Cookies.
fn read_set_cookies(h: http::HeaderMap) -> Vec<Cookie> {
    let set_cookie = h.get("Set-Cookie");
    let cookieCount = {
        match set_cookie {
            None => { 0 }
            Some(h) => { h.len() }
        }
    };
    if cookieCount == 0 {
        return vec![];
    }
    let mut cookies = vec![];
    for line in set_cookie {
        let mut parts: Vec<&str> = line.to_str().unwrap_or_default().trim().split(";").collect();
        if parts.len() == 1 && parts[0] == "" {
            continue;
        }
        parts[0] = parts[0].trim();
        let j: i32 = {
            match parts[0].find("=") {
                None => { -1 }
                Some(v) => { v as i32 }
            }
        };
        if j < 0 {
            continue;
        }


    }
    cookies
}
