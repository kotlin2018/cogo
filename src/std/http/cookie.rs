use std::collections::HashMap;
use std::ops::{Deref, Index};
use http::HeaderValue;
use once_cell::sync::Lazy;
use crate::std::net::textproto;
use crate::std::strings;
use crate::std::time::time::Time;
use crate::std::time::time;

pub struct Cookie {
    pub name: String,
    pub value: String,
    pub path: String,
    pub domain: String,
    pub expires: Time,
    pub raw_expires: String,
    // for reading cookies only
    // MaxAge=0 means no 'Max-Age' attribute specified.
    // MaxAge<0 means delete cookie now, equivalently 'Max-Age: 0'
    // MaxAge>0 means Max-Age attribute present and given in seconds
    pub max_age: i32,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
    pub raw: String,
    pub unparsed: Vec<String>, // Raw text of unparsed attribute-value pairs
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
        let s = textproto::trim_string(line.to_str().unwrap_or_default());
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
            raw: line.to_str().unwrap_or_default().to_string(),
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

const IS_TOKEN_TABLE: Lazy<Vec<char>> = Lazy::new(|| {
    let mut m = Vec::with_capacity(127);
    m.push('!');
    m.push('#');
    m.push('$');
    m.push('%');
    m.push('&');
    m.push('\'');
    m.push('*');
    m.push('+');
    m.push('-');
    m.push('.');
    m.push('0');
    m.push('1');
    m.push('2');
    m.push('3');
    m.push('4');
    m.push('5');
    m.push('6');
    m.push('7');
    m.push('8');
    m.push('9');
    m.push('A');
    m.push('B');
    m.push('C');
    m.push('D');
    m.push('E');
    m.push('F');
    m.push('G');
    m.push('H');
    m.push('I');
    m.push('J');
    m.push('K');
    m.push('L');
    m.push('M');
    m.push('N');
    m.push('O');
    m.push('P');
    m.push('Q');
    m.push('R');
    m.push('S');
    m.push('T');
    m.push('U');
    m.push('V');
    m.push('W');
    m.push('X');
    m.push('Y');
    m.push('Z');
    m.push('^');
    m.push('_');
    m.push('`');
    m.push('a');
    m.push('b');
    m.push('c');
    m.push('d');
    m.push('e');
    m.push('f');
    m.push('g');
    m.push('h');
    m.push('i');
    m.push('j');
    m.push('k');
    m.push('l');
    m.push('m');
    m.push('n');
    m.push('o');
    m.push('p');
    m.push('q');
    m.push('r');
    m.push('s');
    m.push('t');
    m.push('u');
    m.push('v');
    m.push('w');
    m.push('x');
    m.push('y');
    m.push('z');
    m.push('|');
    m.push('~');
    m
});

fn is_token_rune(r: char) -> bool {
    let i = r as usize;
    return (i < IS_TOKEN_TABLE.len()) && IS_TOKEN_TABLE.deref().get(i).is_some();
}


fn valid_cookie_value_byte(b: u8) -> bool {
    return 0x20 <= b && b < 0x7f && b != '"' as u8 && b != ';' as u8 && b != '\\' as u8;
}

fn parse_cookie_value(raw: &str, allow_double_quote: bool) -> (&str, bool) {
    // Strip the quotes, if present.
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