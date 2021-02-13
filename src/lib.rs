#![cfg_attr(not(feature = "debug"), no_std)]
#[cfg(not(feature = "debug"))] extern crate core;

use core::{u8, u16};
use core::cell::Cell;

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
        // println!("Compile {:?} ({:?})", pattern, std::str::from_utf8(pattern));
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
        // println!("Matches {:?} ({:?})", text, std::str::from_utf8(text));
        matches_nfa(self, text)
    }

    pub const fn zeroed() -> Regex {
        Regex {
            pattern: [RegexObj::Unused; MAX_REGEXP_OBJECTS],
            class_buf: [0; MAX_CHAR_CLASS_LEN],
        }
    }
}

#[derive(Clone, Default, Debug)]
struct NfaStateInfo {
    start: Cell<u32>,
    // Option here is actually redundant with pattern[state], but let's keep it for now.
    end: Cell<Option<u32>>,
    next: Cell<Option<u8>>,
    used: Cell<bool>,
}

fn matches_nfa<'t>(regex: &Regex, text: &'t [u8]) -> Option<&'t [u8]> {
    // The n_th element in this array, represents the metatdata about how we entered
    // the n_th state of the nfa. The states form a linked list describing the "priority"
    // which each state has - allowing us to do things like greedy matching.
    let mut state_info: [NfaStateInfo; MAX_REGEXP_OBJECTS] = Default::default();

    let (initial_idx, begin) =
        if regex.pattern[0] == Begin {
            (1, true)
        } else {
            (0, false)
        };
    let head_ptr = Cell::new(None);
    make_epsilon_transitions_and_insert(
        regex,
        NfaStateInfo{ used: Cell::new(true), .. Default::default() },
        initial_idx,
        0,
        &mut &head_ptr,
        &state_info);


    for i in 0.. text.len() {
        #[cfg(feature = "debug")] {
            println!("\nState info:");
            for i in 0.. MAX_REGEXP_OBJECTS {
                if state_info[i].used.get() {
                    println!("\t{} {:?} {:?}", i, regex.pattern[i], state_info[i], );
                }
            }
            println!("Starting at: {:?}", head_ptr);
        }
        // Short circuit if the first state is finished
        let head_state = head_ptr.get()?;
        if Unused == regex.pattern[head_state as usize] {
            let start = state_info[head_state as usize].start.get();
            let end = state_info[head_state as usize].end.get().unwrap();
            return Some(&text[start as usize.. end as usize])
        }

        let mut current_state = head_ptr.get();
        // println!("current state {:?}", current_state);
        let mut new_next_ptr = &head_ptr;
        let new_state_info: [NfaStateInfo; MAX_REGEXP_OBJECTS] = Default::default();
        while let Some(state) = current_state {
            current_state = state_info[state as usize].next.get();
            propogate_state(regex, state_info[state as usize].clone(), state as usize, text[i], i, &mut new_next_ptr, &new_state_info);
        }

        // Add a new state starting on the i+1th character - if there isn't something already in the first state
        if !begin && !new_state_info[0].used.get() {
            make_epsilon_transitions_and_insert(
                regex,
                NfaStateInfo{ used: Cell::new(true), start: Cell::new(i as u32 + 1), .. Default::default() },
                initial_idx,
                i+1,
                &mut new_next_ptr,
                &new_state_info);
        }

        state_info = new_state_info;
    }

    // Search for finished state
    let mut current_state = head_ptr.get();
    while let Some(state) = current_state {
        if matches!(regex.pattern[state as usize], Unused | End) {
            let start = state_info[state as usize].start.get();
            let end = state_info[state as usize].end.get().unwrap_or(text.len() as u32);
            return Some(&text[start as usize.. end as usize])
        }
        current_state = state_info[state as usize].next.get();
    }

    None
}

fn propogate_state<'next>(
    regex: &Regex,
    state_info: NfaStateInfo,
    state: usize,
    c: u8,
    text_idx: usize,
    prev_next_ptr: &mut &'next Cell<Option<u8>>,
    new_state_info: &'next [NfaStateInfo; MAX_REGEXP_OBJECTS]
) {
    // println!("Propogating {} on {} @ {}", state, c, text_idx);
    let obj = regex.pattern[state];
    if obj == Unused {
        return make_epsilon_transitions_and_insert(regex, state_info, state, text_idx + 1, prev_next_ptr, new_state_info);
    }

    let new_state = state + 1;
    if match_one(regex, obj, c) {
        make_epsilon_transitions_and_insert(regex, state_info, new_state, text_idx + 1, prev_next_ptr, new_state_info);
    } // else we failed to transition
}

fn make_epsilon_transitions_and_insert<'next>(
    regex: &Regex,
    state_info: NfaStateInfo,
    state: usize,
    text_idx: usize,
    prev_next_ptr: &mut &'next Cell<Option<u8>>,
    new_state_info: &'next [NfaStateInfo; MAX_REGEXP_OBJECTS]
) {
    // println!("Inserting {} type {:?} info {:?}", state, regex.pattern[state], state_info);
    match regex.pattern[state] {
        Jmp(new_state) =>
            make_epsilon_transitions_and_insert(regex, state_info, new_state as usize, text_idx, prev_next_ptr, new_state_info),
        Split(lhs, rhs) => {
            make_epsilon_transitions_and_insert(regex, state_info.clone(), lhs as usize, text_idx, prev_next_ptr, new_state_info);
            make_epsilon_transitions_and_insert(regex, state_info.clone(), rhs as usize, text_idx, prev_next_ptr, new_state_info);
        }
        obj => {
            // Terminal state, write it unless a higher priority execution already reached this state.
            if !new_state_info[state].used.get() {
                new_state_info[state].used.set(true);
                new_state_info[state].next.set(None);
                new_state_info[state].start.set(state_info.start.get());
                let end =
                    if obj == Unused {
                        Some(state_info.end.get().unwrap_or(text_idx as u32))
                    } else { None };
                new_state_info[state].end.set(end);

                (*prev_next_ptr).set(Some(state as u8));
                *prev_next_ptr = &new_state_info[state].next;
            }
        },
    }
}

fn match_one(regex: &Regex, p: RegexObj, c: u8) -> bool {
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
