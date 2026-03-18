#![no_main]

use std::path::Path;

use csslint_core::FileId;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let source = String::from_utf8_lossy(data);
    let _ = csslint_extractor::extract_styles(FileId::new(1), Path::new("input.vue"), &source);
    let _ = csslint_extractor::extract_styles(FileId::new(2), Path::new("input.svelte"), &source);
    let _ = csslint_extractor::extract_styles(FileId::new(3), Path::new("input.css"), &source);
});
