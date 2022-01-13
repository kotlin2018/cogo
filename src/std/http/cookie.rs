use std::collections::HashMap;
use std::ops::Index;
use http::HeaderValue;
use once_cell::sync::Lazy;
use crate::std::strings;

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
        let j: i32 = strings::index(parts[0], "=");
        if j < 0 {
            continue;
        }
        let name = &parts[0][0..j as usize];
        let value = &parts[0][j as usize + 1..];
        if !isCookieNameValid(name) {
            continue;
        }
    }
    cookies
}


fn isCookieNameValid(raw: &str) -> bool {
    if raw == "" {
        return false;
    }
    return strings.IndexFunc(raw, isNotToken) < 0;
}


fn isNotToken(r: char) -> bool {
    return !IsTokenRune(r);
}

const isTokenTable: Lazy<Vec<char>> = Lazy::new(|| {
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

fn IsTokenRune(r: char) -> bool {
    let i = r as i32;
    return i < len(isTokenTable) && isTokenTable[i];
}