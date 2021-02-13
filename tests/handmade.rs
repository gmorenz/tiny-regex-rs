use std;
use tiny_regex_rs::{matches, Regex, MAX_REGEXP_OBJECTS, MAX_CHAR_CLASS_LEN};

#[test]
fn test_brackets() {
    assert_eq!(Some(&b"bcbc"[..]), matches(b"(bc)+", b"abcbca"));
    assert_eq!(Some(&b"bc"[..]), matches(b"(bc)+", b"bcc"));
    assert_eq!(None, matches(b"(bc)+", b"ccc"));

    assert_eq!(Some(&b"bcdedebcdebc"[..]), matches(b"(bc(de)*)+", b"aadebcdedebcdebcaa"));
}

#[test]
#[ignore] // Infinite recursion :'(
fn nested_quants() {
    assert_eq!(Some(&b"aaaa"[..]), matches(b"a?+", b"aaaaaaaaa"));
}

#[test]
fn test_nearly_empty() {
    assert_eq!(Some(&b""[..]), matches(b"", b""));
    assert_eq!(Some(&b""[..]), matches(b"a?", b""));
}

#[test]
fn simple() {
    assert_eq!(Some(&b"abc"[..]), matches(b"abc", b"abc"));
}

#[test]
fn simple_quant() {
    assert_eq!(Some(&b"aa"[..]), matches(b"a*$", b"Xaa"));
}

#[test]
fn test_size() {
    assert_eq!(std::mem::size_of::<Regex>(), 3 * MAX_REGEXP_OBJECTS + MAX_CHAR_CLASS_LEN);
}