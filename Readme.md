# Tiny Regex Rust Edition

This is a port of [kokke](https://github.com/kokke)'s [tiny-regex-c](https://github.com/kokke/tiny-regex-c) to safe (and `nostd`) rust, with some minor differences outlined the below. Like the original it implements a small subset of regex in a manner suitable for embedded use. We expose both a simple rust api, and the same C interface that tiny-regex-c does.

## Building

To build the rust library, use cargo in it's normal fashion (e.g. `cargo build --release` creates an rlib).

To build the c library run `cargo build --example tiny_regex_rs --release`, the .a file will be `target/release/examples/libtiny_regex_rs.a`. You can use the header file from tiny-regex.c.

## Testing

We run the original tiny-regex-c tests against our c-ffi by using `sh tiny_regex_c_tests.sh`. We pass all of them.

## Differences

Apart from the language difference, the rust edition:

 - Natively uses (ptr, length) strings instead of null terminated strings. The C library runs `strlen` on it's input before passing it to the rust library to convert it to a form suitable for rust.
 - The rust edition properly handles classes of the form `[a-b-]` (that pattern should match the -), where the c version fails it's own tests relating to this.
 - The C edition has a curious behavior where it does not fully wipe state between regex compilations, effecting the behavior of misusing special characters. For example if you first compile a regex like "aba", and then a regex like "a$a", the string "aba" will still match the second regex. The rust edition does not have this behavior.
 - The C edition stores the regex in a static variable and as a result isn't thread safe, and destroys previously compiled regex's when you build a new one. The rust cffi `re_compile` and `re_matchp` functions work similarly. The rust cffi `re_match` and the rust native api store the regex on the stack, and are threadsafe and do not effect previously compiled regex's.
  - Unless the debug feature is enabled (which pulls in all of std) we don't implement re_print in the cffi.

## Performance comparison

They are (unsurprisingly, since it's a pretty direct port right now) very similar. For example, the tiny-regex-c test's take 98.44 seconds to run on my computer when built with the original re.c, and 100.48 seconds when built with the rust ffi (generated via `sh rough_bench.sh`). The rust cffi is likely slightly less efficient because it needs to run strlen on it's inputs before using them.

Binary size is also similar, though *slightly* larger on rust.

```
# Rust binary sizes
-rwxr-xr-x 1 greg users  28K Feb 12 07:00 test1
-rwxr-xr-x 1 greg users  54K Feb 12 07:00 test2
-rwxr-xr-x 1 greg users  17K Feb 12 07:00 test_compile
-rwxr-xr-x 1 greg users  17K Feb 12 07:00 test_rand
-rwxr-xr-x 1 greg users  17K Feb 12 07:00 test_rand_neg
```

```
# C binary sizes
-rwxr-xr-x 1 greg users  27K Feb 12 07:01 test1
-rwxr-xr-x 1 greg users  49K Feb 12 07:01 test2
-rwxr-xr-x 1 greg users  17K Feb 12 07:01 test_compile
-rwxr-xr-x 1 greg users  17K Feb 12 07:01 test_rand
-rwxr-xr-x 1 greg users  17K Feb 12 07:01 test_rand_neg
```