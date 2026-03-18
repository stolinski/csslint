#![no_main]

use std::path::Path;

use csslint_core::FileId;
use csslint_parser::{parse_style_with_options, CssParserOptions};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let source = String::from_utf8_lossy(data);
    let extraction =
        csslint_extractor::extract_styles(FileId::new(5), Path::new("input.css"), &source);
    for style in extraction.styles {
        if let Ok(parsed) = parse_style_with_options(
            &style,
            CssParserOptions {
                enable_recovery: true,
                targets: csslint_core::TargetProfile::Defaults,
            },
        ) {
            let _ = csslint_semantic::build_semantic_model(&parsed);
        }
    }
});
