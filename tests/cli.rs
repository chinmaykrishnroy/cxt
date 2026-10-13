mod common;

use common::{
    assert_success, commit_file, init_git_repo, read_file, run_cxt, temp_dir, write_bytes,
    write_file,
};

fn packed_file_count(output_md: &str) -> usize {
    output_md.matches("### File:").count()
}

#[test]
fn help_and_version_work() {
    let dir = temp_dir("help");

    let help = run_cxt(&dir, &["--help"]);
    assert_success(&help);
    let stdout = String::from_utf8_lossy(&help.stdout);
    assert!(stdout.contains("code-aware context engine"));
    assert!(stdout.contains("--tui"));
    assert!(stdout.contains("--no-minify"));

    let version = run_cxt(&dir, &["--version"]);
    assert_success(&version);
    let stdout = String::from_utf8_lossy(&version.stdout);
    assert!(stdout.contains("cxt"));
}

#[test]
fn directory_scan_respects_gitignore() {
    let dir = temp_dir("gitignore");
    init_git_repo(&dir);

    write_file(&dir, ".gitignore", "ignored.txt\n");
    write_file(&dir, "main.py", "print('hello')\n");
    write_file(&dir, "ignored.txt", "should be ignored\n");

    let out = run_cxt(&dir, &[".", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.py"));
    assert!(md.contains(".gitignore"));
    assert!(
        !md.contains("should be ignored"),
        "ignored.txt should have been skipped by .gitignore"
    );
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn minify_strips_rust_comments() {
    let dir = temp_dir("minify");
    write_file(
        &dir,
        "lib.rs",
        "// This is a secret comment that should be removed\n\
         fn answer() -> i32 { 42 }\n",
    );

    let out = run_cxt(&dir, &[".", "--minify", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("fn answer() -> i32 { 42 }"));
    assert!(
        !md.contains("secret comment"),
        "comment should have been stripped by --minify"
    );
}

#[test]
fn no_minify_overrides_project_default() {
    let dir = temp_dir("no-minify-config");
    write_file(&dir, ".cxt.toml", "default_minify = true\n");
    write_file(
        &dir,
        "lib.rs",
        "// keep this comment when --no-minify is used\nfn answer() -> i32 { 42 }\n",
    );

    let out = run_cxt(&dir, &[".", "--no-minify", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("keep this comment"));
}

#[test]
fn binary_files_are_skipped() {
    let dir = temp_dir("binary");
    write_file(&dir, "main.rs", "fn main() {}\n");
    write_bytes(&dir, "data.bin", b"\x00\x01\x02\x03binary");

    let out = run_cxt(&dir, &[".", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.rs"));
    assert!(!md.contains("data.bin"), "binary file should be skipped");
    assert_eq!(packed_file_count(&md), 1);
}

#[test]
fn invalid_utf8_files_are_skipped_without_aborting_pack() {
    let dir = temp_dir("invalid-utf8");
    write_file(&dir, "good.txt", "readable context\n");
    // No NUL byte: this reaches the text reader and exercises a real read_to_string failure.
    write_bytes(&dir, "bad.txt", &[0xff, 0xfe, 0xfd, b'x']);

    let out = run_cxt(&dir, &[".", "--no-limit", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("good.txt"));
    assert!(!md.contains("bad.txt"), "invalid UTF-8 should be skipped");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("read failed") || stderr.contains("skipped"),
        "the non-fatal read failure should be visible in diagnostics"
    );
}

#[test]
fn large_files_are_skipped_by_default() {
    let dir = temp_dir("large");
    write_file(&dir, "main.rs", "fn main() {}\n");
    // 1.5 MB of text to exceed the default 1 MB limit.
    let big = "x".repeat(1024 * 1024 + 512 * 1024);
    write_file(&dir, "big.txt", &big);

    let out = run_cxt(&dir, &[".", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(md.contains("main.rs"));
    assert!(!md.contains("big.txt"), "large file should be skipped");
    assert!(
        stdout.contains("skipped"),
        "summary should mention skipped files"
    );
}

#[test]
fn trace_python_imports() {
    let dir = temp_dir("python-trace");
    write_file(
        &dir,
        "main.py",
        "from helper import greet\nprint(greet())\n",
    );
    write_file(&dir, "helper.py", "def greet():\n    return 'hi'\n");

    let out = run_cxt(&dir, &["main.py", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.py"));
    assert!(md.contains("helper.py"), "Python import should be traced");
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_javascript_require() {
    let dir = temp_dir("js-trace");
    write_file(
        &dir,
        "main.js",
        "const h = require('./helper');\nh.run();\n",
    );
    write_file(&dir, "helper.js", "module.exports = { run: () => {} };\n");

    let out = run_cxt(&dir, &["main.js", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.js"));
    assert!(md.contains("helper.js"), "JS require should be traced");
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_typescript_import() {
    let dir = temp_dir("ts-trace");
    write_file(&dir, "main.ts", "import { run } from './helper';\nrun();\n");
    write_file(&dir, "helper.ts", "export function run() {}\n");

    let out = run_cxt(&dir, &["main.ts", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.ts"));
    assert!(md.contains("helper.ts"), "TS import should be traced");
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_go_imports() {
    let dir = temp_dir("go-trace");
    write_file(
        &dir,
        "main.go",
        "package main\nimport (\n    \"./util\"\n)\nfunc main() {}\n",
    );
    write_file(&dir, "util/util.go", "package util\nfunc Help() {}\n");

    let out = run_cxt(&dir, &["main.go", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.go"));
    assert!(
        md.contains("util.go") && md.contains("util"),
        "Go import should be traced"
    );
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_java_imports() {
    let dir = temp_dir("java-trace");
    write_file(
        &dir,
        "Main.java",
        "import com.example.Helper;\npublic class Main {}\n",
    );
    write_file(
        &dir,
        "com/example/Helper.java",
        "package com.example;\npublic class Helper {}\n",
    );

    let out = run_cxt(&dir, &["Main.java", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("Main.java"));
    assert!(
        md.contains("Helper.java") && md.contains("example"),
        "Java package import should be traced"
    );
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_c_quoted_includes_and_skip_angle_brackets() {
    let dir = temp_dir("c-trace");
    write_file(
        &dir,
        "main.c",
        "#include <stdio.h>\n#include \"helper.h\"\nint main() { return 0; }\n",
    );
    write_file(&dir, "helper.h", "#pragma once\nvoid helper();\n");
    write_file(
        &dir,
        "helper.c",
        "#include \"helper.h\"\nvoid helper() {}\n",
    );

    let out = run_cxt(&dir, &["main.c", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.c"));
    assert!(md.contains("helper.h"), "C quoted include should be traced");
    assert!(
        !md.contains("helper.c"),
        "helper.c is not directly included by main.c"
    );
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_cpp_quoted_includes_and_skip_angle_brackets() {
    let dir = temp_dir("cpp-trace");
    write_file(
        &dir,
        "main.cpp",
        "#include <iostream>\n#include \"helper.h\"\nint main() { return 0; }\n",
    );
    write_file(&dir, "helper.h", "#pragma once\nint helper();\n");

    let out = run_cxt(&dir, &["main.cpp", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.cpp"));
    assert!(
        md.contains("helper.h"),
        "C++ quoted include should be traced"
    );
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_csharp_using() {
    let dir = temp_dir("cs-trace");
    write_file(
        &dir,
        "Program.cs",
        "using MyApp.Helper;\nclass Program { static void Main() {} }\n",
    );
    write_file(
        &dir,
        "MyApp/Helper.cs",
        "namespace MyApp { class Helper {} }\n",
    );

    let out = run_cxt(&dir, &["Program.cs", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("Program.cs"));
    assert!(
        md.contains("Helper.cs") && md.contains("MyApp"),
        "C# using should be traced"
    );
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_ruby_require() {
    let dir = temp_dir("ruby-trace");
    write_file(&dir, "main.rb", "require './helper'\nputs 'hi'\n");
    write_file(&dir, "helper.rb", "def helper; end\n");

    let out = run_cxt(&dir, &["main.rb", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.rb"));
    assert!(md.contains("helper.rb"), "Ruby require should be traced");
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_php_require() {
    let dir = temp_dir("php-trace");
    write_file(
        &dir,
        "index.php",
        "<?php\nrequire './helper.php';\necho 'hi';\n?>\n",
    );
    write_file(&dir, "helper.php", "<?php\nfunction helper() {}\n?>\n");

    let out = run_cxt(&dir, &["index.php", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("index.php"));
    assert!(md.contains("helper.php"), "PHP require should be traced");
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_python_dotted_import() {
    let dir = temp_dir("python-dotted");
    write_file(
        &dir,
        "main.py",
        "import pkg.helper\nprint(pkg.helper.run())\n",
    );
    write_file(&dir, "pkg/helper.py", "def run():\n    return 'hi'\n");

    let out = run_cxt(&dir, &["main.py", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.py"));
    assert!(
        md.contains("helper.py"),
        "dotted Python import should be traced"
    );
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_python_relative_import() {
    let dir = temp_dir("python-relative");
    write_file(
        &dir,
        "main.py",
        "from . import helper\nprint(helper.run())\n",
    );
    write_file(&dir, "helper.py", "def run():\n    return 'hi'\n");

    let out = run_cxt(&dir, &["main.py", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.py"));
    assert!(
        md.contains("helper.py"),
        "relative Python import should be traced"
    );
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_javascript_dynamic_import() {
    let dir = temp_dir("js-dynamic");
    write_file(
        &dir,
        "main.js",
        "import('./helper.js').then(m => m.run());\n",
    );
    write_file(&dir, "helper.js", "export function run() {}\n");

    let out = run_cxt(&dir, &["main.js", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.js"));
    assert!(
        md.contains("helper.js"),
        "dynamic JS import should be traced"
    );
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_javascript_external_import_is_skipped() {
    let dir = temp_dir("js-external");
    write_file(
        &dir,
        "main.js",
        "import React from 'react';\nexport default App;\n",
    );

    let out = run_cxt(&dir, &["main.js", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.js"));
    assert_eq!(
        packed_file_count(&md),
        1,
        "external dependency should not be packed"
    );
}

#[test]
fn trace_python_external_import_is_skipped() {
    let dir = temp_dir("python-external");
    write_file(&dir, "main.py", "import os\nprint(os.getcwd())\n");

    let out = run_cxt(&dir, &["main.py", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.py"));
    assert_eq!(
        packed_file_count(&md),
        1,
        "external module should not be packed"
    );
}

#[test]
fn trace_c_subdir_quoted_include() {
    let dir = temp_dir("c-subdir");
    write_file(
        &dir,
        "main.c",
        "#include \"lib/helper.h\"\nint main() { return 0; }\n",
    );
    write_file(&dir, "lib/helper.h", "#pragma once\nvoid helper();\n");

    let out = run_cxt(&dir, &["main.c", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.c"));
    assert!(
        md.contains("lib"),
        "subdirectory C include should be traced"
    );
    assert!(md.contains("helper.h"));
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_ruby_require_relative() {
    let dir = temp_dir("ruby-relative");
    write_file(&dir, "main.rb", "require_relative './helper'\nputs 'hi'\n");
    write_file(&dir, "helper.rb", "def helper; end\n");

    let out = run_cxt(&dir, &["main.rb", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.rb"));
    assert!(
        md.contains("helper.rb"),
        "Ruby require_relative should be traced"
    );
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn trace_php_multiple_includes() {
    let dir = temp_dir("php-includes");
    write_file(
        &dir,
        "index.php",
        "<?php\ninclude './helper.php';\ninclude_once './helper2.php';\n?>\n",
    );
    write_file(&dir, "helper.php", "<?php\nfunction h1() {}\n?>\n");
    write_file(&dir, "helper2.php", "<?php\nfunction h2() {}\n?>\n");

    let out = run_cxt(&dir, &["index.php", "--trace", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("index.php"));
    assert!(md.contains("helper.php"));
    assert!(md.contains("helper2.php"));
    assert_eq!(packed_file_count(&md), 3);
}

#[test]
fn redaction_masks_api_key() {
    let dir = temp_dir("redact-key");
    let secret = "a".repeat(24);
    let source = format!(
        r#"const API_KEY: &str = "{}";
fn main() {{}}
"#,
        secret
    );
    write_file(&dir, "main.rs", &source);

    let out = run_cxt(&dir, &[".", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(
        md.contains("[REDACTED_SECRET]"),
        "long API key should be redacted"
    );
    assert!(
        !md.contains(&secret),
        "raw secret should not appear in output"
    );
}

#[test]
fn redaction_masks_token_and_password() {
    let dir = temp_dir("redact-token");
    let token = format!("ghp_{}", "a".repeat(24));
    let password = "hunter2".repeat(3);
    let source = format!(
        "const TOKEN: &str = \"{}\";\nconst PASSWORD: &str = \"{}\";\nfn main() {{}}\n",
        token, password
    );
    write_file(&dir, "main.rs", &source);

    let out = run_cxt(&dir, &[".", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(
        md.contains("[REDACTED_SECRET]"),
        "token and password should be redacted"
    );
    assert!(!md.contains(&token));
    assert!(!md.contains(&password));
}

#[test]
fn short_secret_is_not_redacted() {
    let dir = temp_dir("redact-short");
    write_file(
        &dir,
        "main.rs",
        "const API_KEY: &str = \"short\";\nfn main() {}\n",
    );

    let out = run_cxt(&dir, &[".", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(
        !md.contains("[REDACTED_SECRET]"),
        "short values should not be redacted"
    );
    assert!(md.contains("short"));
}

#[test]
fn diff_mode_packs_only_untracked_files() {
    let dir = temp_dir("git-diff");
    init_git_repo(&dir);
    commit_file(&dir, "tracked.py", "print(1)\n", "track baseline");
    write_file(&dir, "new.py", "print(2)\n");

    let out = run_cxt(&dir, &["--diff", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(
        md.contains("new.py"),
        "untracked file should be packed in diff mode"
    );
    assert!(
        !md.contains("tracked.py"),
        "tracked unchanged file should not appear in diff mode"
    );
    assert_eq!(packed_file_count(&md), 1);
}

#[test]
fn diff_mode_packs_modified_files_and_excludes_ignored_files() {
    let dir = temp_dir("git-diff-modified");
    init_git_repo(&dir);
    commit_file(&dir, ".gitignore", "ignored.txt\n", "add ignore rules");
    commit_file(&dir, "tracked.py", "print(1)\n", "track baseline");
    write_file(&dir, "tracked.py", "print(2)\n");
    write_file(&dir, "ignored.txt", "do not pack\n");

    let out = run_cxt(&dir, &["--diff", "--output", "out.md"]);
    assert_success(&out);

    let md = read_file(&dir, "out.md");
    assert!(md.contains("tracked.py"));
    assert!(md.contains("print(2)"));
    assert!(!md.contains("ignored.txt"));
    assert!(!md.contains("do not pack"));
    assert_eq!(packed_file_count(&md), 1);
}

#[test]
fn dry_run_lists_selected_files() {
    let dir = temp_dir("dry-run");
    write_file(&dir, "main.py", "print('hi')\n");
    write_file(&dir, "README.md", "# hello\n");

    let out = run_cxt(&dir, &[".", "--dry-run"]);
    assert_success(&out);

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("main.py"));
    assert!(stdout.contains("README.md"));
    assert!(
        !stdout.contains("SUCCESS"),
        "dry run should not copy to clipboard"
    );
}

#[test]
fn multiple_explicit_paths_are_packed() {
    let dir = temp_dir("multi-path");
    write_file(&dir, "a.rs", "fn a() {}\n");
    write_file(&dir, "b.py", "def b(): pass\n");
    write_file(&dir, "c.txt", "not selected\n");

    let out = run_cxt(&dir, &["a.rs", "b.py", "--output", "out.md"]);
    assert_success(&out);
    let md = read_file(&dir, "out.md");
    assert!(md.contains("a.rs"));
    assert!(md.contains("b.py"));
    assert!(!md.contains("c.txt"));
    assert_eq!(packed_file_count(&md), 2);
}

#[test]
fn include_ignore_and_extension_filters_work() {
    let dir = temp_dir("filters");
    write_file(&dir, "src/keep.rs", "fn keep() {}\n");
    write_file(&dir, "src/drop.rs", "fn drop_me() {}\n");
    write_file(&dir, "src/keep.py", "print('python')\n");

    let out = run_cxt(
        &dir,
        &[
            ".",
            "--include",
            "src/**",
            "--ignore",
            "**/drop.rs",
            "--ext",
            "rs",
            "--output",
            "out.md",
        ],
    );
    assert_success(&out);
    let md = read_file(&dir, "out.md");
    assert!(md.contains("keep.rs"));
    assert!(!md.contains("drop.rs"));
    assert!(!md.contains("keep.py"));
}

#[test]
fn json_and_xml_formats_work() {
    let dir = temp_dir("formats");
    write_file(&dir, "main.rs", "fn main() {}\n");

    let json = run_cxt(
        &dir,
        &[
            "main.rs",
            "--format",
            "json",
            "--manifest",
            "--output",
            "out.json",
        ],
    );
    assert_success(&json);
    let parsed: serde_json::Value = serde_json::from_str(&read_file(&dir, "out.json")).unwrap();
    assert_eq!(parsed["schema"], "cxt.context.v1");
    assert_eq!(parsed["files"].as_array().unwrap().len(), 1);

    let xml = run_cxt(&dir, &["main.rs", "--format", "xml", "--output", "out.xml"]);
    assert_success(&xml);
    let text = read_file(&dir, "out.xml");
    assert!(text.contains("<context>"));
    assert!(text.contains("<file"));
}

#[test]
fn git_staged_mode_only_packs_staged_changes() {
    let dir = temp_dir("git-staged");
    init_git_repo(&dir);
    commit_file(&dir, "a.py", "print(1)\n", "baseline a");
    commit_file(&dir, "b.py", "print(1)\n", "baseline b");
    write_file(&dir, "a.py", "print(2)\n");
    write_file(&dir, "b.py", "print(3)\n");
    let status = std::process::Command::new("git")
        .args(["add", "a.py"])
        .current_dir(&dir)
        .status()
        .unwrap();
    assert!(status.success());

    let out = run_cxt(&dir, &["--git", "staged", "--output", "out.md"]);
    assert_success(&out);
    let md = read_file(&dir, "out.md");
    assert!(md.contains("a.py"));
    assert!(md.contains("print(2)"));
    assert!(!md.contains("b.py"));
}

#[test]
fn secret_fail_policy_exits_nonzero_without_leaking_secret() {
    let dir = temp_dir("secret-fail");
    write_file(
        &dir,
        "main.rs",
        "const API_KEY: &str = \"abcdef1234567890abcdef\";\n",
    );
    let out = run_cxt(
        &dir,
        &["main.rs", "--secrets", "fail", "--output", "out.md"],
    );
    assert_eq!(out.status.code(), Some(4));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("secret policy 'fail'"));
    assert!(!stderr.contains("abcdef1234567890abcdef"));
    assert!(!dir.join("out.md").exists());
}

#[test]
fn secret_allow_policy_keeps_content() {
    let dir = temp_dir("secret-allow");
    write_file(
        &dir,
        "main.rs",
        "const API_KEY: &str = \"abcdef1234567890abcdef\";\n",
    );
    let out = run_cxt(
        &dir,
        &["main.rs", "--secrets", "allow", "--output", "out.md"],
    );
    assert_success(&out);
    let md = read_file(&dir, "out.md");
    assert!(md.contains("abcdef1234567890abcdef"));
}

#[test]
fn query_mode_ranks_and_limits_context() {
    let dir = temp_dir("query");
    write_file(
        &dir,
        "src/volume_slider.ts",
        "export function synchronizeVolume() { return 'snapcast'; }\n",
    );
    write_file(&dir, "src/unrelated.ts", "export const banana = 1;\n");
    write_file(&dir, "README.md", "generic documentation\n");

    let out = run_cxt(
        &dir,
        &[
            ".",
            "--query",
            "snapcast volume synchronization",
            "--max-files",
            "1",
            "--output",
            "out.md",
        ],
    );
    assert_success(&out);
    let md = read_file(&dir, "out.md");
    assert!(md.contains("volume_slider.ts"));
    assert!(!md.contains("unrelated.ts"));
    assert_eq!(packed_file_count(&md), 1);
}

#[test]
fn path_style_none_hides_source_path() {
    let dir = temp_dir("path-none");
    write_file(&dir, "secret-name.rs", "fn main() {}\n");
    let out = run_cxt(
        &dir,
        &[
            "secret-name.rs",
            "--path-style",
            "none",
            "--output",
            "out.md",
        ],
    );
    assert_success(&out);
    let md = read_file(&dir, "out.md");
    assert!(md.contains("### File 1"));
    assert!(!md.contains("secret-name.rs"));
}

#[test]
fn impact_adds_reverse_dependencies_and_related_tests() {
    let dir = temp_dir("impact");
    init_git_repo(&dir);
    write_file(&dir, "helper.ts", "export const value = 1;\n");
    write_file(
        &dir,
        "app.ts",
        "import { value } from './helper';\nconsole.log(value);\n",
    );
    write_file(
        &dir,
        "helper.test.ts",
        "import { value } from './helper';\ntest('value', () => value);\n",
    );
    let status = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(&dir)
        .status()
        .unwrap();
    assert!(status.success());
    let status = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=cxt-test",
            "-c",
            "user.email=cxt-test@example.com",
            "commit",
            "-q",
            "-m",
            "baseline",
        ])
        .current_dir(&dir)
        .status()
        .unwrap();
    assert!(status.success());
    write_file(&dir, "helper.ts", "export const value = 2;\n");

    let out = run_cxt(
        &dir,
        &["--git", "working", "--impact", "--output", "out.md"],
    );
    assert_success(&out);
    let md = read_file(&dir, "out.md");
    assert!(md.contains("helper.ts"));
    assert!(
        md.contains("app.ts"),
        "reverse dependency should be included"
    );
    assert!(
        md.contains("helper.test.ts"),
        "related test should be included"
    );
}

#[test]
fn sensitive_env_files_are_excluded_by_default() {
    let dir = temp_dir("env-default");
    write_file(&dir, "main.py", "print('safe')\n");
    write_file(&dir, ".env", "SHOULD_NOT_BE_PACKED=1\n");

    let out = run_cxt(&dir, &[".", "--output", "out.md"]);
    assert_success(&out);
    let md = read_file(&dir, "out.md");
    assert!(md.contains("main.py"));
    assert!(!md.contains(".env"));
    assert!(!md.contains("SHOULD_NOT_BE_PACKED=1"));
}

#[test]
fn change_review_includes_deletions_and_hides_sensitive_patches() {
    let dir = temp_dir("review-deletions");
    init_git_repo(&dir);
    commit_file(&dir, "gone.rs", "fn obsolete() {}\n", "baseline");
    commit_file(
        &dir,
        ".env",
        "PRIVATE_CONFIGURATION=old\n",
        "config baseline",
    );
    std::fs::remove_file(dir.join("gone.rs")).unwrap();
    write_file(&dir, ".env", "PRIVATE_CONFIGURATION=changed\n");
    let args = <cxt_core::cli::Args as clap::Parser>::parse_from(["cxt", "--git", "working"]);
    let workspace = cxt_core::review::Workspace::load(&args, &dir).unwrap();
    let deletion = workspace
        .changes
        .iter()
        .find(|c| c.path.ends_with("gone.rs"))
        .unwrap();
    assert_eq!(deletion.status, "D");
    assert!(deletion.diff.contains("-fn obsolete()"));
    let config = workspace
        .changes
        .iter()
        .find(|c| c.path.ends_with(".env"))
        .unwrap();
    assert!(!config.diff.contains("PRIVATE_CONFIGURATION"));
}

#[test]
fn zero_budget_overrides_project_budget() {
    let dir = temp_dir("unlimited-budget");
    write_file(&dir, ".cxt.toml", "max_tokens = 1\n");
    write_file(&dir, "main.rs", "fn main() {}\n");
    let out = run_cxt(&dir, &["main.rs", "--budget", "0", "--stdout"]);
    assert_success(&out);
    assert!(String::from_utf8_lossy(&out.stdout).contains("fn main()"));
}
