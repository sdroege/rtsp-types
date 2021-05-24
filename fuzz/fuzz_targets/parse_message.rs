#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _unused_result = rtsp_types::Message::<&[u8]>::parse(data);
});
