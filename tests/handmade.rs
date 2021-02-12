use tiny_regex_rs::matches;

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