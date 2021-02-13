# Tiny Regex Rust Edition

This [started](https://github.com/gmorenz/tiny-regex-rs/tree/master) as a port of [kokke](https://github.com/kokke)'s [tiny-regex-c](https://github.com/kokke/tiny-regex-c) to safe (and `nostd`) rust, though the internals have changed a bit since then. Like the original it implements a small subset of regex in a manner suitable for embedded use. We expose both a simple rust api, and the same C interface that tiny-regex-c does.

## Building

To build the rust library, use cargo in it's normal fashion (e.g. `cargo build --release` creates an rlib).

To build the c library run `cargo build --example tiny_regex_rs --release`, the .a file will be `target/release/examples/libtiny_regex_rs.a`. You can use the header file from tiny-regex.c.

## Testing

We run the original tiny-regex-c tests against our c-ffi by using `sh tiny_regex_c_tests.sh`. We pass all of them.

We run our own tests via `cargo test --features debug`.

## Differences

|                                   | Rust Edition (nfa branch)                                                  | C Edition                                                                                  |
|-----------------------------------|----------------------------------------------------------------------------|--------------------------------------------------------------------------------------------|
| Regex VM Type                     | NFA                                                                        | Backtracking                                                                               |
| Regex Features                    | C-Edition + brackets for nesting/repeating groups                          | ^, $, *, +, ?, ., \w, \W, \d, \D, \s, \S, character classes                                |
| Worst Case Performance (time)     | Linear (size of regex * size on input)                                     | Exponential                                                                                |
| Best Case Performance (time)      | Pretty good                                                                | Slightly better (but slower than a library like rure)                                      |
| Static Memory Allocation          | 130 bytes per Regex Objec (1 statically allocated in cffi)                 | 520 statically allocated bytes (for a single Regex object equivalent)                      |
| Dynamic Memory Allocation         | 480 bytes (+ normal stack overhead) while evaluating a regex               | Just normal stack overhead                                                                 |
| Intenal String Repr               | (Ptr, Length) pair                                                         | Null Terminated                                                                            |
| Memory safety                     | For sure                                                                   | [Almost for sure](https://github.com/kokke/tiny-regex-c/blob/master/formal_verification.md)|
| Thread Safety                     | In the rust api, and the re_match function in the c api                    | No, compiling a new regex clobbes existing regex's                                         |
| Quirks                            | * counts as 2 regex objects for max regex length, re_print not implemented | `[a-b-]` fails to match `-`, if you first compile `aba` then `a$a`, `aba` will match `a$a` |


## Performance comparison

Since we're using different types of regex engines this is hard to compare, on "test2" in the tiny-regex-c repository the rust cffi version takes `0.22` seconds while the C version takes `6.29` seconds, but that is a test well suited for NFAs. The rust cffi is slightly less efficient than the rust native code because it needs to run strlen on it's inputs before using them.

Binary size is similar between C and rust, though *slightly* larger on rust.

```
# Rust binary sizes
-rwxr-xr-x 1 greg users  27K Feb 13 10:39 test1
-rwxr-xr-x 1 greg users  53K Feb 13 10:39 test2
-rwxr-xr-x 1 greg users  21K Feb 13 10:39 test_compile
-rwxr-xr-x 1 greg users  21K Feb 13 10:39 test_rand
-rwxr-xr-x 1 greg users  21K Feb 13 10:39 test_rand_neg
```

```
# C binary sizes
-rwxr-xr-x 1 greg users  27K Feb 12 07:01 test1
-rwxr-xr-x 1 greg users  49K Feb 12 07:01 test2
-rwxr-xr-x 1 greg users  17K Feb 12 07:01 test_compile
-rwxr-xr-x 1 greg users  17K Feb 12 07:01 test_rand
-rwxr-xr-x 1 greg users  17K Feb 12 07:01 test_rand_neg
```
