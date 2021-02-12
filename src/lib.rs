#![cfg_attr(not(debug), no_std)]
#[cfg(debug)] extern crate core;

use core::{u8, u16};

pub const MAX_REGEXP_OBJECTS: usize = 30;
pub const MAX_CHAR_CLASS_LEN: usize = 40;
pub const MAX_NESTING: usize = 20;

#[derive(Debug)]
pub struct Regex {
    pattern: [RegexObj; MAX_REGEXP_OBJECTS],
    class_buf: [u8; MAX_CHAR_CLASS_LEN],
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum RegexObj {
    Unused,

    // "String feature" objects (that match a char, or a "conceptual" char like beginning/end)
    Dot,
    Begin,
    End,
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

    // End thread
    Jmp(u8),
    Split(u8, u8),
}

// Bring RegexObj variants directly into local namespace....
use RegexObj::*;

impl Regex {
    pub fn compile(pattern: &[u8]) -> Option<Regex> {
        let mut regex = Regex::zeroed();
        let mut class_bufidx = 0;

        // Index into pattern
        let mut i = 0;
        // Index into regex.pattern
        let mut j = 0;
        let mut restart_point = 0;

        let mut brackets = [0; MAX_NESTING];
        let mut brackets_used = 0;

        let shift = |pattern: &mut [RegexObj; MAX_REGEXP_OBJECTS], brackets: &mut [u8; MAX_NESTING], mut bracket_idx, start, end| {
            pattern.copy_within(start.. end, start + 1);
            while bracket_idx > 0 && brackets[bracket_idx] >= start as u8 {
                brackets[bracket_idx] += 1;
                bracket_idx -= 1;
            }
        };

        while i < pattern.len() && j < MAX_REGEXP_OBJECTS {
            regex.pattern[j] = match *pattern.get(i)? {
                // Char like
                b'^' => Begin,
                b'$' => End,
                b'.' => Dot,
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
                    let buf = &mut regex.class_buf[class_bufidx..];
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

                    if len > u8::MAX as usize || class_bufidx > u16::MAX as usize {
                        return None
                    }

                    let begin = class_bufidx as u16;
                    class_bufidx += len;
                    if !negated {
                        CharClass{ begin, len: len as u8 }
                    } else {
                        InvCharClass { begin, len: len as u8 }
                    }
                }

                b'(' => {
                    *brackets.get_mut(brackets_used)? = j as u8;
                    brackets_used += 1;
                    i += 1;
                    continue;
                }
                b')' => {
                    brackets_used -= 1;
                    restart_point = *brackets.get(brackets_used)? as usize;
                    i += 1;
                    continue;
                }

                b'+' => {
                    regex.pattern[j] = Split(restart_point as u8, j as u8 + 1);
                    i += 1;
                    j += 1;
                    continue;
                },
                b'*' => {
                    // Make sure we have space
                    if j + 1 >= regex.pattern.len() {
                        return None;
                    }
                    shift(&mut regex.pattern, &mut brackets, brackets_used, restart_point, j);
                    regex.pattern[restart_point] = Split(restart_point as u8 + 1, j as u8 + 2);
                    regex.pattern[j+1] = Jmp(restart_point as u8);

                    i += 1;
                    j += 2;
                    continue;
                },
                b'?' => {
                    if j >= regex.pattern.len() {
                        return None;
                    }
                    shift(&mut regex.pattern, &mut brackets, brackets_used, restart_point, j);
                    // TODO: This is ungreedy matching the original, aren't most regex engines greedy?
                    regex.pattern[restart_point] = Split(j as u8 + 1, restart_point as u8 + 1);
                    i += 1;
                    j += 1;
                    continue;
                },

                other => Char(other),
            };

            restart_point = j;
            i += 1;
            j += 1;
        }

        Some(regex)
    }

    pub fn matches<'t>(&self, text: &'t [u8]) -> Option<&'t [u8]> {
        match self.pattern[0] {
            // We don't like empty regex's for some reason
            Unused => None,
            Begin => match_beginning(&self, 1, text),
            _ => {
                for start in 0.. text.len() {
                    if let Some(r) = match_beginning(&self, 0, &text[start..]) {
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
            class_buf: [0; MAX_CHAR_CLASS_LEN],
        }
    }
}

fn match_beginning<'t>(regex: &Regex, pattern_idx: usize, text: &'t [u8]) -> Option<&'t [u8]> {
    let end_ptr = match_pattern(regex, pattern_idx, text)?;
    Some(&text[..end_ptr - text.as_ptr() as usize])
}

// Returns ptr to char after last match.
fn match_pattern(regex: &Regex, mut pattern_idx: usize, mut text: &[u8]) -> Option<usize> {
    loop {
        match *regex.pattern.get(pattern_idx)? {
            // Differs from reference?
            Unused => return Some(text.as_ptr() as usize),
            End => return if text.len() == 0 { Some(text.as_ptr() as usize) } else { None },

            Jmp(idx) => pattern_idx = idx as usize,
            Split(lhs, rhs) =>
                if let Some(text) = match_pattern(regex, lhs as usize, text) {
                    return Some(text)
                } else {
                    pattern_idx = rhs as usize
                }

            // Simple patterns
            p if match_one(regex, p, text) => {
                pattern_idx += 1;
                text = &text[1..];
            },

            // Failed to match
            _ => return None
        };
    }
}

fn match_one(regex: &Regex, p: RegexObj, text: &[u8]) -> bool {
    if text.len() == 0 { return false };
    let c = text[0];
    match p {
        Dot => match_dot(c),
        CharClass{begin, len} => match_charclass(regex, begin, len, c),
        InvCharClass{begin, len} => !match_charclass(regex, begin, len, c),
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

fn match_charclass(regex: &Regex, begin: u16, len: u8, c: u8) -> bool {
    let class = &regex.class_buf[begin as usize.. begin as usize + len as usize];
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
