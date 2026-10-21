use traits::doc::{Trait, TraitName, parse, serialize};
use traits::error::ToolFailure;

fn name(value: &str) -> TraitName {
    TraitName::parse(value).expect("valid trait name")
}

fn minimal() -> Trait {
    Trait {
        name: name("delegated-spending"),
        subject: "A payment an agent is considering authorizing.".to_string(),
        requirement: "The payment must fall within delegated spending authority.".to_string(),
        applicability: None,
        purpose: "Prevent unauthorized spending.".to_string(),
        guidance: None,
        context: None,
    }
}

fn parse_error(text: &str) -> ToolFailure {
    parse(text, "/traits/test.md").expect_err("expected a parse failure")
}

const RECOVERY_SUBSTRINGS: [&str; 10] = [
    "Ask the developer:",
    "Do not change files before approval",
    "If declined, leave files unchanged",
    "inspect the entire affected trait store",
    "authoritative requirements, applicability, exceptions, examples",
    "ask a focused question",
    "Validate all files",
    "original arguments",
    "A server restart is not necessary",
    "original operation failed",
];

fn assert_recovery_block(message: &str) {
    assert!(
        message.starts_with("Unable to parse trait /traits/test.md."),
        "{message}"
    );
    for substring in RECOVERY_SUBSTRINGS {
        assert!(
            message.contains(substring),
            "missing {substring:?} in {message}"
        );
    }
}

#[test]
fn missing_opening_delimiter() {
    let failure = parse_error("Body");
    assert!(matches!(failure, ToolFailure::TraitParseError { .. }));
    assert!(
        failure
            .message()
            .contains("Document must start with YAML frontmatter"),
        "{}",
        failure.message()
    );
    assert_recovery_block(failure.message());
}

#[test]
fn unterminated_frontmatter() {
    for text in ["---", "---\nname: example", "---\r\nname: example"] {
        let failure = parse_error(text);
        assert!(
            failure
                .message()
                .contains("Frontmatter is missing its closing delimiter"),
            "{text:?} -> {}",
            failure.message()
        );
        assert_recovery_block(failure.message());
    }
}

#[test]
fn invalid_yaml_reports_line_and_column() {
    let failure = parse_error("---\nname: [\n---\nBody");
    assert!(
        failure.message().contains("Invalid YAML frontmatter"),
        "{}",
        failure.message()
    );
    assert!(failure.message().contains("line "), "{}", failure.message());
    assert!(
        failure.message().contains("column "),
        "{}",
        failure.message()
    );
    assert!(
        failure
            .message()
            .contains("do not assume this is a schema migration"),
        "{}",
        failure.message()
    );
    assert_recovery_block(failure.message());
}

#[test]
fn schema_failure_uses_schema_diagnosis() {
    let failure = parse_error("---\nname: old-format\n---\nPreserve this rule.");
    assert!(
        failure
            .message()
            .contains("does not match the installed Trait schema"),
        "{}",
        failure.message()
    );
    assert_recovery_block(failure.message());
}

#[test]
fn crlf_document_preserves_body_bytes() {
    let text = "---\r\nname: example\r\nsubject: A thing.\r\nrequirement: Do it.\r\npurpose: Stay safe.\r\n---\r\nBody\r\n---\r\n";
    let trait_value = parse(text, "/traits/test.md").expect("parses");
    assert_eq!(trait_value.name, name("example"));
    assert_eq!(trait_value.context.as_deref(), Some("Body\r\n---\r\n"));
}

#[test]
fn body_with_trailing_delimiter_line_is_raw() {
    let text = "---\nname: example\nsubject: A thing.\nrequirement: Do it.\npurpose: Stay safe.\n---\nContext\n\n---\n";
    let trait_value = parse(text, "/traits/test.md").expect("parses");
    assert_eq!(trait_value.context.as_deref(), Some("Context\n\n---\n"));
}

#[test]
fn empty_body_has_no_context() {
    for text in [
        "---\nname: example\nsubject: A thing.\nrequirement: Do it.\npurpose: Stay safe.\n---\n",
        "---\nname: example\nsubject: A thing.\nrequirement: Do it.\npurpose: Stay safe.\n---",
    ] {
        let trait_value = parse(text, "/traits/test.md").expect("parses");
        assert_eq!(trait_value.context, None);
    }
}

#[test]
fn roundtrip_minimal_trait() {
    let trait_value = minimal();
    let text = serialize(&trait_value);
    assert!(!text.contains("context:"), "{text}");
    assert!(!text.contains("applicability:"), "{text}");
    assert!(!text.contains("guidance:"), "{text}");
    assert_eq!(
        parse(&text, "/traits/test.md").expect("parses"),
        trait_value
    );
    let trimmed = text.trim_end_matches('\n');
    assert_eq!(
        parse(trimmed, "/traits/test.md").expect("parses"),
        trait_value
    );
}

#[test]
fn roundtrip_all_fields() {
    let trait_value = Trait {
        applicability: Some("In TypeScript and TSX, except when a synchronous framework contract must be preserved.".to_string()),
        guidance: Some("Use the Effect-aware test APIs.".to_string()),
        context: Some("Supporting context.\n\n---\nExample.\n".to_string()),
        ..minimal()
    };
    let text = serialize(&trait_value);
    assert_eq!(
        parse(&text, "/traits/test.md").expect("parses"),
        trait_value
    );
}

#[test]
fn roundtrip_multiline_context_with_delimiter() {
    let trait_value = Trait {
        context: Some("Supporting context.\n\n---\n\n```diff\n const value = {\n-  bad: true,\n+  good: true,\n }\n```\n".to_string()),
        ..minimal()
    };
    let text = serialize(&trait_value);
    assert_eq!(
        parse(&text, "/traits/test.md").expect("parses"),
        trait_value
    );
}

#[test]
fn parses_folded_and_literal_yaml() {
    let text = concat!(
        "---\n",
        "name: delegated-spending\n",
        "subject: A payment.\n",
        "requirement: >\n",
        "  Stay within\n",
        "  delegated authority.\n",
        "purpose: Protect funds.\n",
        "applicability: |-\n",
        "  Before payment.\n",
        "  Except test transactions.\n",
        "---\n",
        "\n",
        "# Context\n",
        "\n",
        "---\n",
        "Not frontmatter.\n",
    );
    let trait_value = parse(text, "/traits/test.md").expect("parses");
    assert_eq!(
        trait_value.requirement,
        "Stay within delegated authority.\n"
    );
    assert_eq!(
        trait_value.applicability.as_deref(),
        Some("Before payment.\nExcept test transactions.")
    );
    assert_eq!(
        trait_value.context.as_deref(),
        Some("\n# Context\n\n---\nNot frontmatter.\n")
    );
}

#[test]
fn roundtrip_chomping_suffixes() {
    for suffix in ["", "\n", "\n\n"] {
        let trait_value = Trait {
            subject: format!(" A payment proposal\nwith indented context.{suffix}"),
            requirement: format!(
                "Require delegated authority.\n---\nDo not exceed the budget.{suffix}"
            ),
            applicability: Some(format!(
                "Before committing funds.\nExcept explicitly authorized test transactions.{suffix}"
            )),
            purpose: format!("Protect funds.\nEnable autonomous purchases.{suffix}"),
            guidance: Some(format!(
                "Negotiate, decline, or escalate.\nDo not assume permission.{suffix}"
            )),
            context: Some(format!("Supporting context.\n\n---\n\nExample.{suffix}")),
            ..minimal()
        };
        let text = serialize(&trait_value);
        assert!(text.contains("requirement: |"), "{text}");
        assert_eq!(
            parse(&text, "/traits/test.md").expect("parses"),
            trait_value
        );

        let crlf = text.replace('\n', "\r\n");
        let parsed = parse(&crlf, "/traits/test.md").expect("parses crlf");
        let expected = Trait {
            context: trait_value
                .context
                .as_ref()
                .map(|value| value.replace('\n', "\r\n")),
            ..trait_value.clone()
        };
        assert_eq!(parsed, expected);
    }
}

#[test]
fn roundtrip_single_line_quoting_cases() {
    let cases = [
        " leading",
        "trailing ",
        "a: b",
        "a # b",
        "- dash",
        "? question",
        "* star",
        "& anchor",
        "! tag",
        "% directive",
        "@ reserved",
        "` backtick",
        "| pipe",
        "> fold",
        "# comment",
        "\"quoted\"",
        "'single'",
        "[bracket",
        "{brace",
        "true",
        "false",
        "null",
        "~",
        "123",
        "3.14",
        "2024-01-01",
        "2024-01-01T00:00:00Z",
    ];
    for value in cases {
        let trait_value = Trait {
            subject: value.to_string(),
            ..minimal()
        };
        let text = serialize(&trait_value);
        assert_eq!(
            parse(&text, "/traits/test.md").expect("parses"),
            trait_value,
            "value {value:?} in {text}"
        );
    }
}

#[test]
fn blank_values_are_quoted_when_serialized() {
    for value in ["", "   "] {
        let trait_value = Trait {
            subject: value.to_string(),
            ..minimal()
        };
        let text = serialize(&trait_value);
        assert!(text.contains(&format!("subject: \"{value}\"")), "{text}");
    }
}

#[test]
fn rejects_unknown_frontmatter_keys() {
    let header = "name: delegated-spending\nsubject: A payment.\nrequirement: Stay within authority.\npurpose: Protect funds.\n";
    for field in [
        "mime: text/typescript",
        "when: editing",
        "skip: tests",
        "diff: example",
        "body: hidden",
        "context: hidden",
        "priority: 2",
        "extensionMetadata: {}",
    ] {
        let text = format!("---\n{header}{field}\n---\nContext");
        let failure = parse_error(&text);
        assert!(
            failure
                .message()
                .contains("does not match the installed Trait schema"),
            "{field}: {}",
            failure.message()
        );
    }
}

#[test]
fn rejects_body_only_old_format() {
    let failure = parse_error("---\nname: delegated-spending\n---\nAn old body-only requirement.");
    assert!(
        failure
            .message()
            .contains("does not match the installed Trait schema"),
        "{}",
        failure.message()
    );
}

#[test]
fn rejects_missing_required_fields() {
    for header in [
        "name: delegated-spending\nsubject: A payment.\nrequirement: Stay within authority.",
        "name: delegated-spending\nsubject: A payment.\npurpose: Protect funds.",
        "name: delegated-spending\nrequirement: Stay within authority.\npurpose: Protect funds.",
        "subject: A payment.\nrequirement: Stay within authority.\npurpose: Protect funds.",
    ] {
        let failure = parse_error(&format!("---\n{header}\n---"));
        assert!(
            failure
                .message()
                .contains("does not match the installed Trait schema"),
            "{header}: {}",
            failure.message()
        );
    }
}

#[test]
fn rejects_blank_and_wrong_typed_fields() {
    // Render frontmatter with `field` overridden; `suffix` is the text after the colon.
    let build = |field: &str, suffix: &str| -> String {
        let mut out = String::from("---\n");
        for (key, value) in [
            ("name", "delegated-spending"),
            ("subject", "A payment."),
            ("requirement", "Stay within authority."),
            ("purpose", "Protect funds."),
        ] {
            if key != field {
                out.push_str(&format!("{key}: {value}\n"));
            }
        }
        out.push_str(&format!("{field}:{suffix}\n---\n"));
        out
    };

    for field in [
        "name",
        "subject",
        "requirement",
        "applicability",
        "purpose",
        "guidance",
    ] {
        for value in ["\"\"", "\"   \"", "false", "true", "2", "{}", "[]", "~"] {
            let failure = parse_error(&build(field, &format!(" {value}")));
            assert!(
                failure
                    .message()
                    .contains("does not match the installed Trait schema"),
                "{field}: {value} -> {}",
                failure.message()
            );
        }
        // Explicit null (no value) is rejected too.
        let failure = parse_error(&build(field, ""));
        assert!(
            failure
                .message()
                .contains("does not match the installed Trait schema"),
            "{field}: {}",
            failure.message()
        );
    }
}

#[test]
fn rejects_malformed_documents() {
    for text in [
        "---\nsubject: [\n---\nContext",
        "---\nsubject: A payment.\nContext",
        "---\n- not an object\n---",
        "Context without frontmatter",
        "Context before frontmatter\n---\nname: test\n---",
    ] {
        assert!(
            parse(text, "/traits/test.md").is_err(),
            "{text:?} should fail"
        );
    }
}

#[test]
fn rejects_unsafe_names() {
    let long = "a".repeat(81);
    for invalid in [
        "",
        "../escape",
        "with/slash",
        "name.md",
        "Uppercase",
        "two words",
        "snake_case",
        "trait-example",
        long.as_str(),
    ] {
        let text = format!(
            "---\nname: {invalid}\nsubject: A payment.\nrequirement: Stay within authority.\npurpose: Protect funds.\n---\n"
        );
        let failure = parse_error(&text);
        assert!(
            failure
                .message()
                .contains("does not match the installed Trait schema"),
            "{invalid:?}: {}",
            failure.message()
        );
    }
}

#[test]
fn accepts_names_at_the_length_limit() {
    let exact = "a".repeat(80);
    let text = format!(
        "---\nname: {exact}\nsubject: A payment.\nrequirement: Stay within authority.\npurpose: Protect funds.\n---\n"
    );
    let trait_value = parse(&text, "/traits/test.md").expect("parses");
    assert_eq!(trait_value.name.as_str(), exact);
}

#[test]
fn json_names_are_validated_like_stored_names() {
    for invalid in ["../escape", "with/slash", "Uppercase", ""] {
        let error = serde_json::from_value::<TraitName>(serde_json::json!(invalid)).unwrap_err();
        assert!(error.to_string().contains("Invalid trait name"), "{error}");
    }
    assert_eq!(
        serde_json::from_value::<TraitName>(serde_json::json!("delegated-spending")).unwrap(),
        name("delegated-spending")
    );
}

#[test]
fn json_traits_reject_blank_fields_that_could_not_be_stored() {
    let mut value = serde_json::to_value(minimal()).unwrap();
    assert_eq!(
        serde_json::from_value::<Trait>(value.clone()).unwrap(),
        minimal()
    );

    value["subject"] = serde_json::json!("   ");
    let error = serde_json::from_value::<Trait>(value.clone()).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("field subject must not be blank"),
        "{error}"
    );

    value["subject"] = serde_json::json!("A payment.");
    value["guidance"] = serde_json::json!("");
    let error = serde_json::from_value::<Trait>(value).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("field guidance must not be blank"),
        "{error}"
    );
}
