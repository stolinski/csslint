#![forbid(unsafe_code)]

use std::path::Path;

use csslint_core::{Diagnostic, FileId, RuleId, Scope, Severity, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameworkKind {
    Css,
    Vue,
    Svelte,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleLang {
    Css,
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedStyle {
    pub file_id: FileId,
    pub block_index: u32,
    pub content: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub lang: StyleLang,
    pub scope: Scope,
    pub scoped: bool,
    pub module: bool,
    pub framework: FrameworkKind,
}

impl ExtractedStyle {
    pub const fn span(&self) -> Span {
        Span::new(self.start_offset, self.end_offset)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionResult {
    pub styles: Vec<ExtractedStyle>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn extract_styles(file_id: FileId, file_path: &Path, source: &str) -> ExtractionResult {
    let framework = match file_path.extension().and_then(|ext| ext.to_str()) {
        Some("vue") => FrameworkKind::Vue,
        Some("svelte") => FrameworkKind::Svelte,
        _ => FrameworkKind::Css,
    };

    if framework == FrameworkKind::Css {
        return ExtractionResult {
            styles: vec![ExtractedStyle {
                file_id,
                block_index: 0,
                content: source.to_string(),
                start_offset: 0,
                end_offset: source.len(),
                lang: StyleLang::Css,
                scope: Scope::Global,
                scoped: false,
                module: false,
                framework,
            }],
            diagnostics: Vec::new(),
        };
    }

    extract_component_styles(file_id, source, framework)
}

fn extract_component_styles(file_id: FileId, source: &str, framework: FrameworkKind) -> ExtractionResult {
    let mut styles = Vec::new();
    let mut diagnostics = Vec::new();
    let mut cursor = 0;
    let bytes = source.as_bytes();

    while cursor < bytes.len() {
        let Some(tag_start) = find_next_tag_start(bytes, cursor) else {
            break;
        };

        if is_open_tag(bytes, tag_start, b"script") {
            let Some(script_open_end) = find_tag_end(bytes, tag_start + 7) else {
                diagnostics.push(Diagnostic::new(
                    RuleId::from("extractor_unclosed_script_tag"),
                    Severity::Warn,
                    "Unclosed <script> opening tag",
                    Span::new(tag_start, bytes.len()),
                    file_id,
                ));
                break;
            };
            let Some(script_close_start) = find_script_close(bytes, script_open_end + 1) else {
                diagnostics.push(Diagnostic::new(
                    RuleId::from("extractor_unclosed_script_tag"),
                    Severity::Warn,
                    "Missing </script> closing tag",
                    Span::new(tag_start, script_open_end + 1),
                    file_id,
                ));
                break;
            };
            cursor = script_close_start + SCRIPT_CLOSE_TAG.len();
            continue;
        }

        if !is_open_tag(bytes, tag_start, b"style") {
            cursor = tag_start + 1;
            continue;
        }

        let open_start = tag_start;
        let Some(open_end) = find_tag_end(bytes, open_start + 6) else {
            diagnostics.push(Diagnostic::new(
                RuleId::from("extractor_unclosed_style_tag"),
                Severity::Warn,
                "Unclosed <style> opening tag",
                Span::new(open_start, bytes.len()),
                file_id,
            ));
            break;
        };

        let attrs_text = source.get(open_start + 6..open_end).unwrap_or("");
        let attrs = parse_style_attributes(attrs_text);
        let content_start = open_end + 1;

        let Some(close_start) = find_style_close(bytes, content_start) else {
            diagnostics.push(Diagnostic::new(
                RuleId::from("extractor_unclosed_style_tag"),
                Severity::Warn,
                "Missing </style> closing tag",
                Span::new(open_start, open_end + 1),
                file_id,
            ));
            break;
        };

        let close_end = close_start + STYLE_CLOSE_TAG.len();
        if framework == FrameworkKind::Vue && attrs.src {
            diagnostics.push(Diagnostic::new(
                RuleId::from("unsupported_external_style_src"),
                Severity::Warn,
                "Vue <style src> is not resolved in v1",
                Span::new(open_start, open_end + 1),
                file_id,
            ));
            cursor = close_end;
            continue;
        }

        if let Some(lang) = attrs.lang.as_ref() {
            if !lang.eq_ignore_ascii_case("css") {
                diagnostics.push(Diagnostic::new(
                    RuleId::from("unsupported_style_lang"),
                    Severity::Error,
                    format!("Unsupported style language '{lang}' in component style block"),
                    Span::new(open_start, open_end + 1),
                    file_id,
                ));
                cursor = close_end;
                continue;
            }
        }

        let content = source
            .get(content_start..close_start)
            .unwrap_or("")
            .to_string();

        styles.push(ExtractedStyle {
            file_id,
            block_index: styles.len() as u32,
            content,
            start_offset: content_start,
            end_offset: close_start,
            lang: StyleLang::Css,
            scope: scope_for(framework, attrs.scoped, attrs.module),
            scoped: attrs.scoped,
            module: attrs.module,
            framework,
        });

        cursor = close_end;
    }

    ExtractionResult {
        styles,
        diagnostics,
    }
}

const STYLE_CLOSE_TAG: &[u8] = b"</style>";
const SCRIPT_CLOSE_TAG: &[u8] = b"</script>";

fn scope_for(framework: FrameworkKind, scoped: bool, module: bool) -> Scope {
    match framework {
        FrameworkKind::Css => Scope::Global,
        FrameworkKind::Vue if module => Scope::VueModule,
        FrameworkKind::Vue if scoped => Scope::VueScoped,
        FrameworkKind::Vue => Scope::Global,
        FrameworkKind::Svelte => Scope::SvelteScoped,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StyleAttributes {
    scoped: bool,
    module: bool,
    src: bool,
    lang: Option<String>,
}

fn parse_style_attributes(input: &str) -> StyleAttributes {
    let mut scoped = false;
    let mut module = false;
    let mut src = false;
    let mut lang = None;
    let bytes = input.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }

        if index >= bytes.len() {
            break;
        }

        let key_start = index;
        while index < bytes.len()
            && !bytes[index].is_ascii_whitespace()
            && bytes[index] != b'='
            && bytes[index] != b'>'
        {
            index += 1;
        }

        if key_start == index {
            index += 1;
            continue;
        }

        let key = input.get(key_start..index).unwrap_or("").to_ascii_lowercase();

        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }

        let mut value = None;
        if index < bytes.len() && bytes[index] == b'=' {
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                index += 1;
            }

            if index < bytes.len() && (bytes[index] == b'"' || bytes[index] == b'\'') {
                let quote = bytes[index];
                index += 1;
                let value_start = index;
                while index < bytes.len() && bytes[index] != quote {
                    index += 1;
                }
                value = Some(input.get(value_start..index).unwrap_or("").to_string());
                if index < bytes.len() && bytes[index] == quote {
                    index += 1;
                }
            } else {
                let value_start = index;
                while index < bytes.len()
                    && !bytes[index].is_ascii_whitespace()
                    && bytes[index] != b'>'
                {
                    index += 1;
                }
                value = Some(input.get(value_start..index).unwrap_or("").to_string());
            }
        }

        match key.as_str() {
            "scoped" => scoped = true,
            "module" => module = true,
            "src" => src = true,
            "lang" => lang = value,
            _ => {}
        }
    }

    StyleAttributes {
        scoped,
        module,
        src,
        lang,
    }
}

fn find_next_tag_start(bytes: &[u8], start: usize) -> Option<usize> {
    bytes
        .iter()
        .enumerate()
        .skip(start)
        .find_map(|(index, byte)| (*byte == b'<').then_some(index))
}

fn find_style_close(bytes: &[u8], start: usize) -> Option<usize> {
    let mut index = start;
    while index + STYLE_CLOSE_TAG.len() <= bytes.len() {
        if bytes[index] == b'<'
            && starts_with_ignore_ascii_case(&bytes[index..], STYLE_CLOSE_TAG)
        {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn find_script_close(bytes: &[u8], start: usize) -> Option<usize> {
    let mut index = start;
    while index + SCRIPT_CLOSE_TAG.len() <= bytes.len() {
        if bytes[index] == b'<'
            && starts_with_ignore_ascii_case(&bytes[index..], SCRIPT_CLOSE_TAG)
        {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn is_open_tag(bytes: &[u8], index: usize, tag_name: &[u8]) -> bool {
    if bytes.get(index) != Some(&b'<') {
        return false;
    }

    let tag_end = index + 1 + tag_name.len();
    if tag_end > bytes.len() {
        return false;
    }

    for (offset, expected) in tag_name.iter().enumerate() {
        if !bytes[index + 1 + offset].eq_ignore_ascii_case(expected) {
            return false;
        }
    }

    let boundary = bytes.get(tag_end).copied();
    if !boundary.map(is_style_boundary).unwrap_or(true) {
        return false;
    }

    true
}

fn find_tag_end(bytes: &[u8], mut index: usize) -> Option<usize> {
    let mut quote: Option<u8> = None;

    while index < bytes.len() {
        let current = bytes[index];
        if let Some(active_quote) = quote {
            if current == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        if current == b'"' || current == b'\'' {
            quote = Some(current);
            index += 1;
            continue;
        }

        if current == b'>' {
            return Some(index);
        }

        index += 1;
    }

    None
}

fn starts_with_ignore_ascii_case(haystack: &[u8], needle: &[u8]) -> bool {
    if haystack.len() < needle.len() {
        return false;
    }

    haystack
        .iter()
        .zip(needle.iter())
        .take(needle.len())
        .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

fn is_style_boundary(byte: u8) -> bool {
    byte.is_ascii_whitespace() || byte == b'>'
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use csslint_core::FileId;

    use crate::{extract_styles, ExtractionResult, FrameworkKind};

    #[test]
    fn extracts_css_as_single_block() {
        let source = "body { color: red; }";
        let result = extract_styles(FileId::new(1), Path::new("base.css"), source);

        assert_eq!(result.styles.len(), 1);
        assert_eq!(result.styles[0].block_index, 0);
        assert_eq!(result.styles[0].start_offset, 0);
        assert_eq!(result.styles[0].end_offset, source.len());
        assert_eq!(result.styles[0].framework, FrameworkKind::Css);
        assert_eq!(result.styles[0].content, source);
    }

    #[test]
    fn extracts_vue_blocks_in_source_order() {
        let source = "<template/>\n<style scoped>.a { color: red; }</style>\n<style>.b { color: blue; }</style>";
        let result = extract_styles(FileId::new(2), Path::new("Comp.vue"), source);

        assert_eq!(result.styles.len(), 2);
        assert_eq!(result.styles[0].block_index, 0);
        assert_eq!(result.styles[1].block_index, 1);
        assert_eq!(slice_from_style(source, &result, 0), result.styles[0].content);
        assert_eq!(slice_from_style(source, &result, 1), result.styles[1].content);
    }

    #[test]
    fn skips_vue_src_blocks_with_warning() {
        let source = "<style src=\"./remote.css\">.ignored { color: red; }</style>";
        let result = extract_styles(FileId::new(9), Path::new("Comp.vue"), source);

        assert!(result.styles.is_empty());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].rule_id.as_str(), "unsupported_external_style_src");
    }

    #[test]
    fn skips_unsupported_lang_blocks_with_error() {
        let source = "<style lang=\"scss\">$color: red;</style>";
        let result = extract_styles(FileId::new(10), Path::new("Comp.vue"), source);

        assert!(result.styles.is_empty());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].rule_id.as_str(), "unsupported_style_lang");
    }

    #[test]
    fn marks_vue_module_variants_as_module_blocks() {
        let source = "<style module>.a { color: red; }</style>\n<style module=\"named\">.b { color: blue; }</style>";
        let result = extract_styles(FileId::new(11), Path::new("Comp.vue"), source);

        assert_eq!(result.styles.len(), 2);
        assert!(result.styles[0].module);
        assert!(result.styles[1].module);
    }

    #[test]
    fn ignores_style_like_text_inside_script_blocks() {
        let source = "<script>const css = '<style>.fake { color: red; }</style>';</script>\n<style>.real { color: blue; }</style>";
        let result = extract_styles(FileId::new(12), Path::new("Comp.vue"), source);

        assert_eq!(result.styles.len(), 1);
        assert!(result.styles[0].content.contains(".real"));
    }

    #[test]
    fn extracts_svelte_style_block() {
        let source = "<script>let n = 1;</script>\n<style>h1 { font-size: 2rem; }</style>";
        let result = extract_styles(FileId::new(3), Path::new("Comp.svelte"), source);

        assert_eq!(result.styles.len(), 1);
        assert_eq!(result.styles[0].framework, FrameworkKind::Svelte);
        assert_eq!(slice_from_style(source, &result, 0), result.styles[0].content);
    }

    #[test]
    fn reports_missing_script_close_as_controlled_warning() {
        let source = "<script>const css = '<style>.fake { color: red; }</style>';\n<style>.real { color: blue; }</style>";
        let result = extract_styles(FileId::new(20), Path::new("Comp.vue"), source);

        assert!(result.styles.is_empty());
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].rule_id.as_str(), "extractor_unclosed_script_tag");
        assert_eq!(result.diagnostics[0].severity.as_str(), "warn");
    }

    fn slice_from_style(source: &str, result: &ExtractionResult, index: usize) -> String {
        source
            .get(result.styles[index].start_offset..result.styles[index].end_offset)
            .unwrap_or("")
            .to_string()
    }
}
