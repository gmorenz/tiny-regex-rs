#![cfg_attr(not(feature = "debug"), no_std)]

use tiny_regex_rs::*;
use core::{ptr, slice};

unsafe fn strlen(mut cstr: *const u8) -> usize {
    let mut i = 0;
    while *cstr != 0 {
        i += 1;
        cstr = cstr.offset(1);
    }
    i
}

#[no_mangle]
pub unsafe extern "C" fn re_compile(pattern: *const u8) -> *const Regex {
    static mut REGEX: Regex = Regex::zeroed();

    let slice = slice::from_raw_parts(pattern, strlen(pattern));

    match Regex::compile(slice) {
        Some(r) => REGEX = r,
        None => return ptr::null()
    }
    &REGEX as *const Regex
}

#[no_mangle]
pub unsafe extern "C" fn re_matchp(regex: &Regex, text: *const u8, match_length: *mut i32) -> i32 {
    let slice = slice::from_raw_parts(text, strlen(text));
    if let Some(output) = regex.matches(slice) {
        *match_length = output.len() as i32;
        (output.as_ptr() as usize - text as usize) as i32
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn re_match(pattern: *const u8, text: *const u8, match_length: *mut i32) -> i32 {
    let pattern_slice = slice::from_raw_parts(pattern, strlen(pattern));
    Regex::compile(pattern_slice)
        .map(|regex| re_matchp(&regex, text, match_length))
        .unwrap_or(-1)
}

#[doc(hidden)]
#[no_mangle]
#[allow(unused_variables)]
pub unsafe extern "C" fn re_print(regex: &Regex) {
    #[cfg(feature = "debug")]
    println!("\t{:?}", regex);
}

#[panic_handler]
#[cfg(not(feature = "debug"))]
unsafe fn abort_on_panic(_info: &core::panic::PanicInfo) -> ! {
    // Force abort on stable nostd rust by panicing inside a panic :'(
    struct A;

    impl Drop for A {
        #[inline(always)]
        fn drop(&mut self) { panic!() }
    }

    let _a = A;
    panic!()
}