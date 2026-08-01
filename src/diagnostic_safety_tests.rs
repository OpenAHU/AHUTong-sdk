fn assert_omits(source_name: &str, source: &str, forbidden: &[&str]) {
    for fragment in forbidden {
        assert!(
            !source.contains(fragment),
            "{source_name} reintroduced unsafe diagnostic fragment: {fragment}"
        );
    }
}

#[test]
fn production_diagnostics_do_not_embed_sensitive_runtime_values() {
    assert_omits(
        "core.rs",
        include_str!("core.rs"),
        &["cookies: {:?}", "cookies: {}"],
    );
    assert_omits(
        "jni.rs",
        include_str!("jni.rs"),
        &[
            "for user: {}",
            "save path from Java: {}",
            "Root Error: {:?}",
            " from {} to {}",
            "{} -> {}",
            ": {:?}",
            "e.to_string()",
        ],
    );
    assert_omits(
        "updater.rs",
        include_str!("updater.rs"),
        &[
            "url={}, save_path={}",
            "ip_url: {}",
            "Creating file at {}",
            "calculated: {}, expected: {}",
            "config response: {}",
            "Parsed config: {:?}",
            "ipUrl={}",
        ],
    );
    assert_omits(
        "parser.rs",
        include_str!("utils/parser.rs"),
        &[
            "Content: {}",
            "Script Content Head: {}",
            "HTML Head: {}",
            "HTML: {}",
            "name={}",
        ],
    );
    assert_omits(
        "persistence.rs",
        include_str!("persistence.rs"),
        &[
            "path.display()",
            "invalid {kind}: {value}",
            "box {box_name}",
            "key {key}",
        ],
    );
    assert_omits(
        "ffi.rs",
        include_str!("ffi.rs"),
        &["error.to_string()", "Some(e.to_string())", "{error}"],
    );
    assert_omits(
        "wasm.rs",
        include_str!("wasm.rs"),
        &["format!(\"{:#}\", e)"],
    );
    assert_omits(
        "adwmh.rs",
        include_str!("data/api/adwmh.rs"),
        &["body.chars()", "body={}"],
    );
}

#[test]
fn production_logs_do_not_use_debug_formatting() {
    let debug_format = regex::Regex::new(r"\{(?:[A-Za-z_][A-Za-z0-9_]*)?:[^}]*\?\}")
        .expect("valid debug-format guard");
    let dynamic_error_argument = regex::Regex::new(
        r"(?s)(?:log::)?(?:trace|debug|info|warn|error)!\s*\([^;]*,\s*&?(?:e|err|error)\b",
    )
    .expect("valid dynamic-error guard");

    for (source_name, source) in [
        ("core.rs", include_str!("core.rs")),
        ("jni.rs", include_str!("jni.rs")),
        ("updater.rs", include_str!("updater.rs")),
        ("parser.rs", include_str!("utils/parser.rs")),
    ] {
        assert!(
            !debug_format.is_match(source),
            "{source_name} must not interpolate arbitrary Debug output"
        );
        assert!(
            !dynamic_error_argument.is_match(source),
            "{source_name} must not interpolate an arbitrary error value"
        );
    }
}
