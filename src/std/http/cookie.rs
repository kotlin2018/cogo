use std::collections::HashMap;
use std::fmt::Write;
use std::ops::{Deref, Index};
use http::{HeaderMap, HeaderValue};
use once_cell::sync::Lazy;
use crate::hash_map;
use crate::std::net::textproto;
use crate::std::strings;
use crate::std::time::time::{Time, TimeFormat};
use crate::std::time::time;

#[derive(Eq, PartialEq, Debug)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub path: String,
    pub domain: String,
    pub expires: Time,
    pub raw_expires: String,
    /// for reading cookies only
    /// MaxAge=0 means no 'Max-Age' attribute specified.
    /// MaxAge<0 means delete cookie now, equivalently 'Max-Age: 0'
    /// MaxAge>0 means Max-Age attribute present and given in seconds
    pub max_age: i32,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
    pub raw: String,
    pub unparsed: Vec<String>, // Raw text of unparsed attribute-value pairs
}

/// SameSite allows a server to define a cookie attribute making it impossible for
/// the browser to send this cookie along with cross-site requests. The main
/// goal is to mitigate the risk of cross-origin information leakage, and provide
/// some protection against cross-site request forgery attacks.
//
/// See https://tools.ietf.org/html/draft-ietf-httpbis-cookie-same-site-00 for details.
pub type SameSite = i32;

pub const SameSiteDefaultMode: SameSite = 1;
pub const SameSiteLaxMode: SameSite = 1;
pub const SameSiteStrictMode: SameSite = 1;
pub const SameSiteNoneMode: SameSite = 1;

/// readSetCookies parses all "Set-Cookie" values from
/// the header h and returns the successfully parsed Cookies.
fn read_set_cookies(h: http::HeaderMap) -> Vec<Cookie> {
    let set_cookie = h.get_all("Set-Cookie");
    let set_cookie = {
        let mut v = vec![];
        for x in set_cookie {
            v.push(x);
        }
        v
    };
    let cookieCount = set_cookie.len();
    if cookieCount == 0 {
        return vec![];
    }
    let mut cookies = vec![];
    for value in set_cookie {
        let line = value.to_str().unwrap_or_default();
        let s = textproto::trim_string(line);
        let mut parts: Vec<&str> = s.split(";").collect();
        if parts.len() == 1 && parts[0] == "" {
            continue;
        }
        let mut parts = {
            let mut data = Vec::with_capacity(parts.capacity());
            for x in parts {
                data.push(x.to_string());
            }
            data
        };
        parts[0] = textproto::trim_string(&parts[0]);
        let j: i32 = strings::index(&parts[0], "=");
        if j < 0 {
            continue;
        }
        let name = &parts[0][0..j as usize];
        let value = &parts[0][j as usize + 1..];
        if !is_cookie_name_valid(name) {
            continue;
        }
        let (value, ok) = parse_cookie_value(value, true);
        if !ok {
            continue;
        }
        let mut c = Cookie {
            name: name.to_string(),
            value: value.to_string(),
            path: "".to_string(),
            domain: "".to_string(),
            expires: Time::default(),
            raw_expires: "".to_string(),
            max_age: 0,
            secure: false,
            http_only: false,
            same_site: 0,
            raw: line.to_string(),
            unparsed: vec![],
        };
        for i in 0..parts.len() {
            parts[i] = textproto::trim_string(&parts[i]);
            if parts[i].len() == 0 {
                continue;
            }
            let mut attr = parts[i].as_str();
            let mut val = "";
            if let Some(j) = attr.find("=") {
                attr = &attr[0..j];
                val = &attr[j + 1..];
            }
            let lowerAttr = attr.to_lowercase();
            let (val, ok) = parse_cookie_value(val, false);
            if !ok {
                c.unparsed.push(parts[i].clone());
            }
            match lowerAttr.as_str() {
                "samesite" => {
                    let lowerVal = val.to_lowercase();
                    match lowerVal.as_str() {
                        "lax" => {
                            c.same_site = SameSiteLaxMode;
                        }
                        "strict" => {
                            c.same_site = SameSiteStrictMode;
                        }
                        "none" => {
                            c.same_site = SameSiteNoneMode;
                        }
                        _ => {
                            c.same_site = SameSiteDefaultMode;
                        }
                    }
                    continue;
                }
                "secure" => {
                    c.secure = true;
                    continue;
                }
                "httponly" => {
                    c.http_only = true;
                    continue;
                }
                "domain" => {
                    c.domain = val.to_string();
                }
                "max-age" => {
                    let secs = val.parse();
                    if secs.is_err() {
                        break;
                    }
                    let mut secs: i32 = secs.unwrap();
                    if secs != 0 && val.starts_with("0") {
                        break;
                    }
                    if secs <= 0 {
                        secs = -1
                    }
                    c.max_age = secs;
                    continue;
                }
                "expires" => {
                    c.raw_expires = val.to_string();
                    let mut exptime = Time::parse(time::RFC1123, val);
                    if let Err(_) = exptime {
                        exptime = Time::parse(time::RFC1123, val);
                        if exptime.is_err() {
                            c.expires = time::Time::default();
                            break;
                        }
                    }
                    c.expires = exptime.unwrap().utc();
                }
                "path" => {
                    c.path = val.to_string();
                    continue;
                }
                _ => {}
            }
            c.unparsed.push(parts[i].clone());
        }
        cookies.push(c);
    }
    cookies
}

/// SetCookie adds a Set-Cookie header to the provided ResponseWriter's headers.
/// The provided cookie must have a valid Name. Invalid cookies may be
/// silently dropped.
pub fn set_cookie(cookie: &mut Cookie) {
    cookie.string();
}

/// readCookies parses all "Cookie" values from the header h and
/// returns the successfully parsed Cookies.
//
/// if filter isn't empty, only cookies of that name are returned
fn read_cookies(h: HeaderMap, filter: &str) -> Vec<Cookie> {
    let lines = {
        let mut v = vec![];
        for x in h.get_all("Cookie") {
            v.push(x);
        }
        v
    };
    if lines.is_empty() || lines.len() == 0 {
        return vec![];
    }
    let mut cookies = Vec::with_capacity(lines.len());
    for line in lines {
        let line = line.to_str().unwrap_or_default();
        let mut line = textproto::trim_string(line);
        let mut part: String;
        if line.len() > 0 {
            if let Some(splitIndex) = line.find(";") {
                part = line[0..splitIndex].to_string();
                line = line[splitIndex + 1..].to_string();
            } else {
                part = line.to_string();
                line = String::new();
            }
            part = textproto::trim_string(part.as_str());
            if part.is_empty() {
                continue;
            }
            let mut name = part.clone();
            let mut val = String::new();
            if let Some(j) = part.find("=") {
                val = name[(j + 1)..].to_string();
                name = name[0..j].to_string();
            }
            if !is_cookie_name_valid(&name) {
                continue;
            }
            if filter != "" && filter != name {
                continue;
            }
            let (val, ok) = parse_cookie_value(&val, true);
            if !ok {
                continue;
            }
            cookies.push(Cookie {
                name,
                value: val.to_string(),
                path: "".to_string(),
                domain: "".to_string(),
                expires: Default::default(),
                raw_expires: "".to_string(),
                max_age: 0,
                secure: false,
                http_only: false,
                same_site: 0,
                raw: "".to_string(),
                unparsed: vec![],
            });
        } else {
            continue;
        }
    }
    cookies
}

/// validCookieExpires reports whether v is a valid cookie expires-value.
fn valid_cookie_expires(t: &Time) -> bool {
/// IETF RFC 6265 Section 5.1.1.5, the year must not be less than 1601
    return t.year() >= 1601;
}

/// String returns the serialization of the cookie for use in a Cookie
/// header (if only Name and Value are set) or a Set-Cookie response
/// header (if other fields are set).
/// If c is nil or c.Name is invalid, the empty string is returned.
impl Cookie {
    pub fn string(&self) -> String {
        if is_cookie_name_valid(self.name.as_str()) {
            return String::new();
        }
        /// extraCookieLength derived from typical length of cookie attributes
        /// see RFC 6265 Sec 4.1.
        const extraCookieLength: i32 = 110;
        let mut b = String::with_capacity(self.name.len() + self.value.len() + self.path.len() + extraCookieLength as usize);
        b.write_str(&self.name);
        b.write_str("=");
        b.write_str(sanitize_cookie_value(self.value.as_str()).as_str());
        if self.path.len() > 0 {
            b.write_str("; Path=");
            b.write_str(sanitize_cookie_path(&self.path).as_str());
        }
        if self.domain.len() > 0 {
            if valid_cookie_domain(&self.domain) {
                /// A c.Domain containing illegal characters is not
                /// sanitized but simply dropped which turns the cookie
                /// into a host-only cookie. A leading dot is okay
                /// but won't be sent.
                let mut d = self.domain.clone();
                if d.starts_with('.') {
                    d = d[1..].to_string();
                }
                b.write_str("; Domain=");
                b.write_str(d.as_str());
            } else {
                log::info!("net/http: invalid Cookie.Domain {}; dropping domain attribute", self.domain);
            }
        }
        if valid_cookie_expires(&self.expires) {
            b.write_str("; Expires=");
            b.write_str(self.expires.utc().format(TimeFormat).as_str());
        }
        if self.max_age > 0 {
            b.write_str("; Max-Age=");
            b.write_str(self.max_age.to_string().as_str());
        } else if self.max_age < 0 {
            b.write_str("; Max-Age=0");
        }
        if self.http_only {
            b.write_str("; HttpOnly");
        }
        if self.secure {
            b.write_str("; Secure");
        }
        match self.same_site {
            SameSiteDefaultMode => {
                // Skip, default mode is obtained by not emitting the attribute.
            }
            SameSiteNoneMode => {
                b.write_str("; SameSite=None");
            }
            SameSiteLaxMode => {
                b.write_str("; SameSite=Lax");
            }
            SameSiteStrictMode => {
                b.write_str("; SameSite=Strict");
            }
            _ => {}
        }
        b.to_string()
    }
}


/// sanitize_cookie_value produces a suitable cookie-value from v.
/// https://tools.ietf.org/html/rfc6265#section-4.1.1
/// cookie-value      = *cookie-octet / ( DQUOTE *cookie-octet DQUOTE )
/// cookie-octet      = %x21 / %x23-2B / %x2D-3A / %x3C-5B / %x5D-7E
///           ; US-ASCII characters excluding CTLs,
///           ; whitespace DQUOTE, comma, semicolon,
///           ; and backslash
/// We loosen this as spaces and commas are common in cookie values
/// but we produce a quoted cookie-value if and only if v contains
/// commas or spaces.
/// See https://golang.org/issue/7243 for the discussion.
fn sanitize_cookie_value(v: &str) -> String {
    let v = sanitize_or_warn("Cookie.Value", valid_cookie_value_byte, v);
    if v.is_empty() {
        return v;
    }
    if v.find(' ').is_some() || v.find(',').is_some() {
        return format!("\"{}\"", v);
    }
    return v;
}

/// isCookieDomainName reports whether s is a valid domain name or a valid
/// domain name with a leading dot '.'.  It is almost a direct copy of
/// package net's isDomainName.
fn is_cookie_domain_name(s: &str) -> bool {
    let mut s = s.to_string();
    if s.len() == 0 {
        return false;
    }
    if s.len() > 255 {
        return false;
    }
    if s.starts_with('.') {
        // A cookie a domain attribute may start with a leading dot.
        s = s[1..].to_string();
    }
    let mut s = s.into_bytes();
    let mut last = '.' as u8;
    let mut ok = false; /// Ok once we've seen a letter.
    let mut partlen = 0;
    for i in 0..s.len() {
        let c = s[i];
        if 'a' as u8 <= c && c <= 'z' as u8 || 'A' as u8 <= c && c <= 'Z' as u8 {
            // No '_' allowed here (in contrast to package net).
            ok = true;
            partlen += 1;
        } else if '0' as u8 <= c && c <= '9' as u8 {
            // fine
            partlen += 1;
        } else if c as u8 == '-' as u8 {
            // Byte before dash cannot be dot.
            if last == '.' as u8 {
                return false;
            }
            partlen += 1;
        } else if c == '.' as u8 {
            // Byte before dot cannot be dot, dash.
            if last == '.' as u8 || last == '-' as u8 {
                return false;
            }
            if partlen > 63 || partlen == 0 {
                return false;
            }
            partlen = 0;
        } else {
            return false;
        }
        last = c;
    }
    if last == '-' as u8 || partlen > 63 {
        return false;
    }
    return ok;
}

fn valid_cookie_domain(v: &str) -> bool {
    if is_cookie_domain_name(v) {
        return true;
    }
    //TODO net.ParseIP(v) != nil
    if !v.is_empty()
        && !v.contains(":") {
        return true;
    }
    return false;
}

fn valid_cookie_value_byte(b: u8) -> bool {
    return 0x20 <= b && b < 0x7f && b != '"' as u8 && b != ';' as u8 && b != '\\' as u8;
}

fn sanitize_or_warn(field_name: &str, valid: fn(u8) -> bool, v: &str) -> String {
    let mut ok = true;
    let cs = v.chars();
    for c in cs {
        if valid(c as u8) {
            continue;
        }
        log::info!("net/http: invalid byte {} in {}; dropping invalid bytes", c, field_name);
        ok = false;
    }
    if ok {
        return v.to_string();
    }
    let mut buf = String::with_capacity(v.len());
    let cs = v.chars();
    for c in cs {
        if valid(c as u8) {
            buf.push(c);
        }
    }
    return buf;
}

//isCookieNameValid
fn is_cookie_name_valid(raw: &str) -> bool {
    if raw == "" {
        return false;
    }
    return strings::index_func(raw, is_not_token) < 0;
}

fn is_not_token(r: char) -> bool {
    return !is_token_rune(r);
}

static IS_TOKEN_TABLE: Lazy<HashMap<char, bool>> = Lazy::new(|| {
    hash_map! {
    '!':  true,
	'#':  true,
	'$':  true,
	'%':  true,
	'&':  true,
	'\'': true,
	'*':  true,
	'+':  true,
	'-':  true,
	'.':  true,
	'0':  true,
	'1':  true,
	'2':  true,
	'3':  true,
	'4':  true,
	'5':  true,
	'6':  true,
	'7':  true,
	'8':  true,
	'9':  true,
	'A':  true,
	'B':  true,
	'C':  true,
	'D':  true,
	'E':  true,
	'F':  true,
	'G':  true,
	'H':  true,
	'I':  true,
	'J':  true,
	'K':  true,
	'L':  true,
	'M':  true,
	'N':  true,
	'O':  true,
	'P':  true,
	'Q':  true,
	'R':  true,
	'S':  true,
	'T':  true,
	'U':  true,
	'W':  true,
	'V':  true,
	'X':  true,
	'Y':  true,
	'Z':  true,
	'^':  true,
	'_':  true,
	'`':  true,
	'a':  true,
	'b':  true,
	'c':  true,
	'd':  true,
	'e':  true,
	'f':  true,
	'g':  true,
	'h':  true,
	'i':  true,
	'j':  true,
	'k':  true,
	'l':  true,
	'm':  true,
	'n':  true,
	'o':  true,
	'p':  true,
	'q':  true,
	'r':  true,
	's':  true,
	't':  true,
	'u':  true,
	'v':  true,
	'w':  true,
	'x':  true,
	'y':  true,
	'z':  true,
	'|':  true,
	'~':  true,
    }
});

fn is_token_rune(r: char) -> bool {
    let i = r as u8;
    return ((i as usize) < 127) && IS_TOKEN_TABLE.get(&r).is_some();
}

fn parse_cookie_value(raw: &str, allow_double_quote: bool) -> (&str, bool) {
    /// Strip the quotes, if present.
    let mut raw = raw;
    if allow_double_quote && raw.len() > 1 && raw.starts_with('"') && raw.ends_with('"') {
        raw = raw.trim_matches('"');
    }
    for x in raw.chars() {
        if !valid_cookie_value_byte(x as u8) {
            return ("", false);
        }
    }
    return (raw, true);
}

fn valid_cookie_path_byte(b: u8) -> bool {
    return 0x20 <= b && b < 0x7f && b != (';' as u8);
}

/// path-av           = "Path=" path-value
/// path-value        = <any CHAR except CTLs or ";">
fn sanitize_cookie_path(v: &str) -> String {
    return sanitize_or_warn("Cookie.Path", valid_cookie_path_byte, v);
}

#[cfg(test)]
mod test {
    use std::ops::Deref;
    use http::{HeaderMap, HeaderValue};
    use crate::std::http::cookie::{Cookie, is_cookie_name_valid, read_cookies};
    use crate::std::lazy::sync::Lazy;

    static readCookiesTests: Lazy<Vec<(HeaderMap, &'static str, Vec<Cookie>)>> = Lazy::new(|| {
        let mut h1 = HeaderMap::new();
        h1.insert("Cookie", HeaderValue::from_str("Cookie-1=v$1").unwrap());
        h1.append("Cookie", HeaderValue::from_str("c2=v2").unwrap());

        let mut h2 = Cookie {
            name: "Cookie-1".to_string(),
            value: "v$1".to_string(),
            path: "".to_string(),
            domain: "".to_string(),
            expires: Default::default(),
            raw_expires: "".to_string(),
            max_age: 0,
            secure: false,
            http_only: false,
            same_site: 0,
            raw: "".to_string(),
            unparsed: vec![],
        };
        let mut h3 = Cookie {
            name: "c2".to_string(),
            value: "v2".to_string(),
            path: "".to_string(),
            domain: "".to_string(),
            expires: Default::default(),
            raw_expires: "".to_string(),
            max_age: 0,
            secure: false,
            http_only: false,
            same_site: 0,
            raw: "".to_string(),
            unparsed: vec![],
        };
        vec![
            (h1, "", vec![h2, h3])
        ]
    });

    #[test]
    fn TestIndexFunc() {
        assert_eq!(is_cookie_name_valid("Cookie-1"), true);
    }

    #[test]
    fn TestReadCookies() {
        let mut i = 0;
        for tt in readCookiesTests.deref() {
            for n in 0..2 {
                let c = read_cookies(tt.0.clone(), tt.1);
                for x in &c {
                    println!("cookie:{}", x.name);
                }
                assert_eq!(tt.2.len(), c.len());
                let mut idx = 0;
                for x in &tt.2 {
                    assert_eq!(x, tt.2.get(idx).unwrap());
                    idx += 1;
                }
            }
            i += 1;
        }
    }

    #[test]
    fn TestCookieSanitizePath() {
        let tests = vec![("/path", "/path"), ("/path with space/", "/path with space/"), ("/just;no;semicolon\x00orstuff/", "/justnosemicolonorstuff/")];

        for (_, tt) in tests {
            // let got =
        }
    }
}