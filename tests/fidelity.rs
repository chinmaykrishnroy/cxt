#[allow(dead_code)]
mod common;

use common::{assert_success, run_cxt, temp_dir, write_file};

#[test]
fn xml_rejects_forbidden_characters_before_overwriting_output() {
    let dir = temp_dir("xml-controls");
    write_file(&dir, "data.txt", "valid prefix\u{1}invalid XML\n");
    write_file(&dir, "out.xml", "existing output");
    let out = run_cxt(
        &dir,
        &["data.txt", "--format", "xml", "--output", "out.xml"],
    );
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        std::fs::read_to_string(dir.join("out.xml")).unwrap(),
        "existing output"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("XML 1.0"));
}

#[test]
fn minification_preserves_documentation_and_tool_directives() {
    let rust =
        "/// Public contract\n///\n/// ```rust\n/// answer();\n/// ```\npub fn answer() {}\n";
    assert_eq!(
        cxt_core::minifier::minify_syntax(rust, tree_sitter_rust::language()),
        rust
    );
    let js = "// @ts-check\nconst value = 1;\n// ordinary comment\n";
    let out = cxt_core::minifier::minify_syntax(js, tree_sitter_javascript::language());
    assert!(out.contains("// @ts-check"));
    assert!(!out.contains("ordinary comment"));
}

#[test]
fn xml_export_roundtrips_mixed_line_endings_and_delimiters() {
    let dir = temp_dir("xml-fidelity");
    let source = "α <&> \" ' ]]>\r\n\tsecond\rthird\n";
    write_file(&dir, "data.txt", source);
    let out = run_cxt(&dir, &["data.txt", "--format", "xml", "--stdout"]);
    assert_success(&out);
    let xml = String::from_utf8(out.stdout).unwrap();
    let doc = roxmltree::Document::parse(&xml).unwrap();
    let content = doc
        .descendants()
        .find(|n| n.has_tag_name("content"))
        .unwrap();
    assert_eq!(content.text(), Some(source));
}

#[test]
fn markdown_file_headers_preserve_backticks_in_names() {
    let dir = temp_dir("markdown-path");
    write_file(&dir, "a`b.rs", "fn main() {}\n");
    let out = run_cxt(&dir, &["a`b.rs", "--stdout"]);
    assert_success(&out);
    let markdown = String::from_utf8(out.stdout).unwrap();
    let codes: Vec<_> = pulldown_cmark::Parser::new(&markdown)
        .filter_map(|event| match event {
            pulldown_cmark::Event::Code(text) => Some(text.into_string()),
            _ => None,
        })
        .collect();
    assert_eq!(codes, vec!["a`b.rs"]);
}

#[test]
fn markdown_export_keeps_long_fences_inside_their_file() {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};
    let dir = temp_dir("markdown-fences");
    let source = "before\n````````\n### Not a file header\n````````\nafter\n";
    write_file(&dir, "first.md", source);
    write_file(&dir, "second.rs", "fn second() {}\n");
    let out = run_cxt(&dir, &["first.md", "second.rs", "--stdout"]);
    assert_success(&out);
    let markdown = String::from_utf8(out.stdout).unwrap();
    let mut blocks = Vec::new();
    let mut current = None;
    for event in Parser::new(&markdown) {
        match event {
            Event::Start(Tag::CodeBlock(_)) => current = Some(String::new()),
            Event::Text(text) => {
                if let Some(content) = &mut current {
                    content.push_str(&text);
                }
            }
            Event::End(TagEnd::CodeBlock) => blocks.push(current.take().unwrap()),
            _ => {}
        }
    }
    assert_eq!(
        blocks,
        vec![format!("{source}\n"), "fn second() {}\n\n".to_string()]
    );
}

#[test]
fn comment_stripping_preserves_token_boundaries_and_line_breaks() {
    let source = "int/**/value = 1;\nint next = 2;/* note\r\nmore */int last = 3;\n";
    let out = cxt_core::minifier::strip_comments(source, tree_sitter_c::language());
    assert!(out.contains("int value"));
    assert!(out.contains("\r\n"));
    assert!(!out.contains("note"));
}

#[test]
fn minified_cli_export_preserves_multiline_literal_bytes() {
    let dir = temp_dir("literal-fidelity");
    let source = "const message = `First line  \r\n\r\nThird line`;\r\n\r\n// remove this comment\r\nconsole.log(message);\r\n";
    write_file(&dir, "message.js", source);
    let out = run_cxt(
        &dir,
        &["message.js", "--minify", "--format", "json", "--stdout"],
    );
    assert_success(&out);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let content = json["files"][0]["content"].as_str().unwrap();
    assert!(content.contains("`First line  \r\n\r\nThird line`"));
    assert!(!content.contains("remove this comment"));
}

#[test]
fn syntax_compaction_preserves_literal_tokens_across_languages() {
    fn tokens(node: tree_sitter::Node, source: &str, out: &mut Vec<(String, String)>) {
        if node.kind().contains("comment") {
            return;
        }
        if node.child_count() == 0 {
            out.push((
                node.kind().to_string(),
                source[node.byte_range()].to_string(),
            ));
        } else {
            for index in 0..node.child_count() {
                tokens(node.child(index).unwrap(), source, out);
            }
        }
    }
    let cases = [
        (
            "js",
            "const text = `α  \n\nβ`;\n\n// comment\nconsole.log(text);\n",
        ),
        (
            "ts",
            "const text: string = `α  \n\nβ`;\n\n// comment\nconsole.log(text);\n",
        ),
        (
            "rs",
            "fn main() {\nlet text = r#\"α  \n\nβ\"#;\n\n// comment\nprintln!(\"{}\", text);\n}\n",
        ),
        (
            "py",
            "text = \"\"\"α  \n\nβ\"\"\"\n\n# comment\nprint(text)\n",
        ),
        (
            "go",
            "package main\n\nfunc main() {\ntext := `α  \n\nβ`\n\n// comment\nprintln(text)\n}\n",
        ),
        ("c", "#define VALUE 1\n\nint/**/value = VALUE;\n"),
        (
            "rb",
            "text = <<~MSG\nα  \n\nβ\nMSG\n\n# comment\nputs text\n",
        ),
        (
            "php",
            "<?php\n$text = <<<'MSG'\nα  \n\nβ\nMSG;\n\n// comment\necho $text;\n",
        ),
    ];
    for (ext, source) in cases {
        let language = cxt_core::tracer::get_language(ext).unwrap();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(language).unwrap();
        let before = parser.parse(source, None).unwrap();
        assert!(!before.root_node().has_error(), "invalid {ext} fixture");
        let output = cxt_core::minifier::minify_syntax(source, language);
        let after = parser.parse(&output, None).unwrap();
        assert!(!after.root_node().has_error(), "invalid transformed {ext}");
        let mut expected = Vec::new();
        let mut actual = Vec::new();
        tokens(before.root_node(), source, &mut expected);
        tokens(after.root_node(), &output, &mut actual);
        assert_eq!(actual, expected, "changed syntax tokens in {ext}");
    }
}

#[test]
fn unknown_grammar_and_invalid_syntax_are_not_compacted() {
    let dir = temp_dir("minify-fallback");
    let source = "message: |\n  first  \n\n  third\n\n";
    write_file(&dir, "data.yaml", source);
    let out = run_cxt(
        &dir,
        &["data.yaml", "--minify", "--format", "json", "--stdout"],
    );
    assert_success(&out);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["files"][0]["content"], source);
    let invalid = "const x = `unterminated\n\n// not a removable comment\n";
    assert_eq!(
        cxt_core::minifier::minify_syntax(invalid, tree_sitter_javascript::language()),
        invalid
    );
}

#[test]
fn minification_cannot_hide_original_comment_secrets_from_fail_policy() {
    let dir = temp_dir("minify-secret-policy");
    let secret = format!("ghp_{}", "a".repeat(24));
    write_file(&dir, "main.rs", &format!("// {secret}\nfn main() {{}}\n"));
    let out = run_cxt(
        &dir,
        &["main.rs", "--minify", "--secrets", "fail", "--stdout"],
    );
    assert_eq!(out.status.code(), Some(4));
    assert!(out.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&out.stderr).contains(&secret));
}

#[test]
fn json_roundtrips_unicode_controls_and_long_lines_without_minification() {
    let dir = temp_dir("json-source-bytes");
    let source = format!("α\u{1}\r\n\t{}\n", "text<&>`".repeat(10000));
    write_file(&dir, "data.txt", &source);
    let out = run_cxt(&dir, &["data.txt", "--format", "json", "--stdout"]);
    assert_success(&out);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["files"][0]["content"], source);
}
