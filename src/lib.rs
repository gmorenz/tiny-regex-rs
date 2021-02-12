#![cfg_attr(not(debug), no_std)]
#[cfg(debug)] extern crate core;

use core::{u8, u16};

pub const MAX_REGEXP_OBJECTS: usize = 30;
pub const MAX_CHAR_CLASS_LEN: usize = 40;
pub struct Regex {
    pattern: [RegexObj; MAX_REGEXP_OBJECTS],
    ccl_buf: [u8; MAX_CHAR_CLASS_LEN],
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum RegexObj {
    Unused,
    Dot,
    Begin,
    End,
    QuestionMark,
    Star,
    Plus,
    Char(u8),
    // Use short indicies so we can pack this enum into a u32 :)
    CharClass{ begin: u16, len: u8 },
    InvCharClass{ begin: u16, len: u8 },
    Digit,
    NotDigit,
    Alpha,
    NotAlpha,
    Whitespace,
    NotWhitespace,
}

// Bring RegexObj variants directly into local namespace....
use RegexObj::*;

impl Regex {
    pub fn compile(pattern: &[u8]) -> Option<Regex> {
        let mut regex = Regex::zeroed();
        let mut ccl_bufidx = 0;

        // Index into pattern
        let mut i = 0;
        // Index into regex.pattern
        let mut j = 0;

        while i < pattern.len() && j < MAX_REGEXP_OBJECTS {
            regex.pattern[j] = match *pattern.get(i)? {
                b'^' => Begin,
                b'$' => End,
                b'.' => Dot,
                b'*' => Star,
                b'+' => Plus,
                b'?' => QuestionMark,
                b'\\' => {
                    i += 1;
                    match *pattern.get(i)? {
                        b'd' => Digit,
                        b'D' => NotDigit,
                        b'w' => Alpha,
                        b'W' => NotAlpha,
                        b's' => Whitespace,
                        b'S' => NotWhitespace,
                        // TODO: Shouldn't this be Metachar?
                        other => Char(other)
                    }
                }
                // Character class
                b'[' => {
                    i += 1;
                    let buf = &mut regex.ccl_buf[ccl_bufidx..];
                    let negated = *pattern.get(i)? == b'^';
                    if negated { i += 1 };

                    let mut next;
                    let mut len = 0;
                    while { next = *pattern.get(i)?; next != b']' } {
                        if next == b'\\' {
                            *buf.get_mut(len)? = next;
                            len += 1;
                            i += 1;
                            next = *pattern.get(i)?;
                        }
                        *buf.get_mut(len)? = next;
                        len += 1;
                        i += 1;
                    }

                    if len > u8::MAX as usize || ccl_bufidx > u16::MAX as usize {
                        return None
                    }

                    let begin = ccl_bufidx as u16;
                    ccl_bufidx += len;
                    if !negated {
                        CharClass{ begin, len: len as u8 }
                    } else {
                        InvCharClass { begin, len: len as u8 }
                    }
                }
                other => Char(other),
            };

            i += 1;
            j += 1;
        }

        Some(regex)
    }

    pub fn matches<'t>(&self, text: &'t [u8]) -> Option<&'t [u8]> {
        match self.pattern[0] {
            // We don't like empty regex's for some reason
            Unused => None,
            Begin => match_beginning(&self.ccl_buf, &self.pattern[1..], text),
            _ => {
                for start in 0.. text.len() {
                    if let Some(r) = match_beginning(&self.ccl_buf, &self.pattern, &text[start..]) {
                        return Some(r)
                    }
                }
                None
            }
        }
    }

    pub const fn zeroed() -> Regex {
        Regex {
            pattern: [RegexObj::Unused; MAX_REGEXP_OBJECTS],
            ccl_buf: [0; MAX_CHAR_CLASS_LEN],
        }
    }
}

fn match_beginning<'t>(buf: &[u8; MAX_CHAR_CLASS_LEN], pattern: &[RegexObj], text: &'t [u8]) -> Option<&'t [u8]> {
    let end_ptr = match_pattern(buf, pattern, text)?;
    Some(&text[..end_ptr - text.as_ptr() as usize])
}

// Returns ptr to char after last match.
fn match_pattern(buf: &[u8; MAX_CHAR_CLASS_LEN], mut pattern: &[RegexObj], mut text: &[u8]) -> Option<usize> {
    loop {
        match (*pattern.get(0)?, *pattern.get(1)?) {
            // Differs from reference?
            (Unused, _) => return Some(text.as_ptr() as usize),
            (End, Unused) => return if text.len() == 0 { Some(text.as_ptr() as usize) } else { None },

            (obj, QuestionMark) => return match_questionmark(buf, obj, pattern.get(2..)?, text),
            (obj, Star) => return match_repeat(buf, obj, 0, pattern.get(2..)?, text),
            (obj, Plus) => return match_repeat(buf, obj, 1, pattern.get(2..)?, text),

            // Simple patterns
            (p, _) if match_one(buf, p, text) => {
                pattern = &pattern[1..];
                text = &text[1..];
            },

            // Failed to match
            _ => return None
        };
    }
}

fn match_questionmark(buf: &[u8; MAX_CHAR_CLASS_LEN], question: RegexObj, pattern: &[RegexObj], mut text: &[u8]) -> Option<usize> {
    if let Some(end) = match_pattern(buf, pattern, text) {
        return Some(end)
    }

    if match_one(buf, question, text) {
        text = text.get(1..)?;
        match_pattern(buf, pattern, text)
    } else {
        None
    }
}

fn match_repeat(buf: &[u8; MAX_CHAR_CLASS_LEN], obj: RegexObj, min_repeat: usize, pattern: &[RegexObj], text: &[u8]) -> Option<usize> {
    let mut max_l = 0;
    while text.len() > max_l && match_one(buf, obj, &text[max_l..]) {
        max_l += 1;
    }
    for i in (min_repeat ..= max_l).rev() {
        if let Some(end) = match_pattern(buf, pattern, &text[i..]) {
            return Some(end)
        }
    }
    None
}

fn match_one(buf: &[u8; MAX_CHAR_CLASS_LEN], p: RegexObj, text: &[u8]) -> bool {
    if text.len() == 0 { return false };
    let c = text[0];
    match p {
        Dot => match_dot(c),
        CharClass{begin, len} => match_charclass(buf, begin, len, c),
        InvCharClass{begin, len} => !match_charclass(buf, begin, len, c),
        Digit => match_digit(c),
        NotDigit => !match_digit(c),
        Alpha => match_alphanum(c),
        NotAlpha => !match_alphanum(c),
        Whitespace => match_whitespace(c),
        NotWhitespace => !match_whitespace(c),
        Char(expected_c) => c == expected_c,
        // Unexpected (this would be a bug in the original!)
        _ => false,
    }
}

fn match_dot(_: u8) -> bool {
    // TODO: cfg RE_DOT_MATCHES_NEWLINE? Or?
    true
}

fn match_charclass(buf: &[u8; MAX_CHAR_CLASS_LEN], begin: u16, len: u8, c: u8) -> bool {
    let class = &buf[begin as usize.. begin as usize + len as usize];
    let mut i = 0;
    while i < class.len() {
        if match_range(c, &class[i..]) {
            return true
        }
        else if class[i] == b'\\' {
            if i + 1 >= class.len() {
                // Malformed class :'(
                // Returning false here is unfortunate, because it is "wrong" for inverse char classes
                // but I guess we treat is as "unspecified regex behavior"
                return false;
            }
            if match class[i + 1] {
                b'd' => match_digit(c),
                b'D' => !match_digit(c),
                b'w' => match_alphanum(c),
                b'W' => !match_alphanum(c),
                b's' => match_whitespace(c),
                b'S' => !match_whitespace(c),
                _ => c == class[i + 1]
            } {
                return true
            }
            i += 2;
            continue;
        }
        // Slight variation from the original here, it would return false on
        // a-b- contains -, while we (and grep) return true
        else if (i == 0 || i + 1 == class.len()) && class[i] == b'-' && c == b'-' {
            return true
        }
        else if c != b'-' && c == class[i] {
            return true
        }
        i += 1;
    }
    false
}

fn match_range(c: u8, range: &[u8]) -> bool {
    range.len() >= 3
        && c != b'-' // Who knew
        && range[0] != b'-' // Weird rule, but I gues a--b is weird otherwise
        && range[1] == b'-'
        && c >= range[0]
        && c <= range[2]
}

fn match_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}

fn match_alphanum(c: u8) -> bool {
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z') || c == b'_' || match_digit(c)
}

fn match_whitespace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0xc /* \f */ | 0x0b /* \v */)
}

pub fn matches<'t>(pattern: &[u8], text: &'t [u8]) -> Option<&'t [u8]> {
    Regex::compile(pattern).and_then(|regex| regex.matches(text))
}
