use crate::*;

const SPAN: nu::Span = nu::Span::unknown();

fn span(start: usize, end: usize) -> nu::Span {
    nu::Span::new(start, end)
}

#[test]
fn a_parse_error_keeps_its_message_and_label_without_debug_text() {
    let error = nu::ParseError::OutputMismatch(
        nu_protocol::Type::Int,
        "string".into(),
        span(10, 20),
    );
    let val = NuValError::from(&error);
    assert_eq!(
        val.kind,
        NuValErrorKind::Parse(ParseErrorKind::OutputMismatch)
    );
    assert_eq!(val.msg, "Command output doesn't match int.");
    assert_eq!(val.code.as_deref(), Some("nu::parser::output_type_mismatch"));
    assert_eq!(val.code.as_deref(), val.kind.code());
    let [label] = val.labels.as_slice() else {
        panic!("one label: {:?}", val.labels);
    };
    assert_eq!(
        label.text.as_deref(),
        Some("expected int, but command outputs string")
    );
    assert_eq!((label.span.start, label.span.end), (10, 20));
}

#[test]
fn a_shell_error_keeps_the_label_and_help_its_display_drops() {
    let error = nu::ShellError::ExternalCommand {
        label: "Command `plugin` not found".into(),
        help: "`plugin` is neither a Nushell built-in or a known external command".into(),
        span: span(0, 6),
    };
    let val = NuValError::from(&error);
    assert_eq!(
        val.kind,
        NuValErrorKind::Shell(ShellErrorKind::ExternalCommand)
    );
    assert_eq!(val.msg, "External command failed");
    assert_eq!(
        val.labels[0].text.as_deref(),
        Some("Command `plugin` not found")
    );
    assert_eq!(
        val.help.as_deref(),
        Some("`plugin` is neither a Nushell built-in or a known external command")
    );
    assert_eq!(val.severity, NuValSeverity::Error);
}

#[test]
fn every_std_io_kind_holds_nushells_code_and_message() {
    use std::io::ErrorKind as K;
    let kinds = [
        K::NotFound,
        K::PermissionDenied,
        K::ConnectionRefused,
        K::ConnectionReset,
        K::HostUnreachable,
        K::NetworkUnreachable,
        K::ConnectionAborted,
        K::NotConnected,
        K::AddrInUse,
        K::AddrNotAvailable,
        K::NetworkDown,
        K::BrokenPipe,
        K::AlreadyExists,
        K::WouldBlock,
        K::NotADirectory,
        K::IsADirectory,
        K::DirectoryNotEmpty,
        K::ReadOnlyFilesystem,
        K::StaleNetworkFileHandle,
        K::InvalidInput,
        K::InvalidData,
        K::TimedOut,
        K::WriteZero,
        K::StorageFull,
        K::NotSeekable,
        K::QuotaExceeded,
        K::FileTooLarge,
        K::ResourceBusy,
        K::ExecutableFileBusy,
        K::Deadlock,
        K::CrossesDevices,
        K::TooManyLinks,
        K::InvalidFilename,
        K::ArgumentListTooLong,
        K::Interrupted,
        K::Unsupported,
        K::UnexpectedEof,
        K::OutOfMemory,
        K::Other,
    ];
    for kind in kinds {
        let io_kind = nu::IoKind::from_std(kind);
        let error = nu::ShellError::Io(nu_protocol::shell_error::io::IoError::new_internal(
            io_kind,
            "the context",
        ));
        let val = NuValError::from(&error);
        let NuValErrorKind::Io(IoErrorKind::Std(std_kind)) = val.kind else {
            panic!("{kind:?} isn't a std io kind: {:?}", val.kind);
        };
        assert_eq!(Some(std_kind.code()), val.code.as_deref(), "{kind:?}'s code");
        assert_eq!(std_kind.message(), io_kind.to_string(), "{kind:?}'s message");
        assert_eq!(val.cause.as_deref(), Some("the context"));
        assert!(val.source.is_some(), "{kind:?} carries its location");
    }
}

#[test]
fn a_dns_error_takes_its_own_kind() {
    let error = nu::ShellError::Network(nu::NetworkError::Dns(
        nu_protocol::shell_error::network::DnsError {
            kind: nu::DnsErrorKind::NoName,
            span: span(1, 2),
            query: nu_protocol::Spanned {
                item: "example.invalid".into(),
                span: span(3, 4),
            },
        },
    ));
    let val = NuValError::from(&error);
    assert_eq!(
        val.kind,
        NuValErrorKind::Network(NetworkErrorKind::DnsNoName)
    );
    assert_eq!(val.code.as_deref(), val.kind.code());
    assert_eq!(val.labels.len(), 2);
}

#[test]
fn a_job_error_takes_its_own_kind() {
    let error = nu::ShellError::Job(nu::JobError::NotFound {
        span: SPAN,
        id: nu_protocol::JobId::new(3),
    });
    let val = NuValError::from(&error);
    assert_eq!(val.kind, NuValErrorKind::Job(JobErrorKind::NotFound));
    assert_eq!(val.msg, "Job 3 not found");
    assert_eq!(val.code.as_deref(), val.kind.code());
}

#[test]
fn a_generic_error_keeps_its_code_help_and_inner() {
    let error = nu::ShellError::Generic(
        nu_protocol::shell_error::generic::GenericError::new("A title", "the label", span(5, 9))
            .with_code("my::code")
            .with_help("try again")
            .with_inner([nu::ShellError::DivisionByZero { span: span(6, 7) }]),
    );
    let val = NuValError::from(&error);
    assert_eq!(val.kind, NuValErrorKind::Generic);
    assert_eq!(val.msg, "A title");
    assert_eq!(val.code.as_deref(), Some("my::code"));
    assert_eq!(val.help.as_deref(), Some("try again"));
    assert_eq!(val.labels[0].text.as_deref(), Some("the label"));
    assert_eq!(
        val.inner[0].kind,
        NuValErrorKind::Shell(ShellErrorKind::DivisionByZero)
    );
    assert!(val.source.is_none());
}

#[test]
fn an_internal_generic_error_carries_its_location() {
    let error = nu::ShellError::Generic(
        nu_protocol::shell_error::generic::GenericError::new_internal("A title", "the label"),
    );
    let val = NuValError::from(&error);
    let source = val.source.as_ref().expect("a location");
    let label = &val.labels[0];
    assert_eq!(
        (label.span.start, label.span.end),
        (0, source.text.len() as u64)
    );
}

#[test]
fn a_config_error_and_the_shell_error_it_wraps_take_their_own_kinds() {
    let error = nu::ShellError::InvalidConfig {
        errors: vec![
            nu::ConfigError::UnknownOption {
                path: "$env.config.nope".into(),
                span: span(1, 5),
            },
            nu::ConfigError::ShellError(nu::ShellError::DivisionByZero { span: span(2, 3) }),
        ],
    };
    let val = NuValError::from(&error);
    assert_eq!(
        val.kind,
        NuValErrorKind::Shell(ShellErrorKind::InvalidConfig)
    );
    let kinds: Vec<_> = val.inner.iter().map(|e| e.kind).collect();
    assert_eq!(
        kinds,
        vec![
            NuValErrorKind::Config(ConfigErrorKind::UnknownOption),
            NuValErrorKind::Shell(ShellErrorKind::DivisionByZero),
        ]
    );
}

#[test]
fn a_chain_is_its_first_error_and_wraps_its_later_links() {
    let first = nu::ShellError::DivisionByZero { span: span(4, 5) }.into_chained(span(0, 9));
    let val = NuValError::from(&first);
    assert_eq!(
        val.kind,
        NuValErrorKind::Shell(ShellErrorKind::DivisionByZero)
    );
    let later = first.into_chained(span(10, 20));
    let val = NuValError::from(&later);
    assert_eq!(val.kind, NuValErrorKind::Chained);
    assert_eq!(val.code.as_deref(), Some("chained_error"));
    assert_eq!(
        val.inner[0].kind,
        NuValErrorKind::Shell(ShellErrorKind::DivisionByZero)
    );
}

#[test]
fn a_warning_is_a_warning() {
    let warning = nu::ShellWarning::Deprecated {
        dep_type: "Flag".into(),
        label: "use --other".into(),
        span: span(0, 4),
        help: None,
        report_mode: nu_protocol::ReportMode::FirstUse,
    };
    let val = NuValError::from(&warning);
    assert_eq!(
        val.kind,
        NuValErrorKind::ShellWarning(ShellWarningKind::Deprecated)
    );
    assert_eq!(val.severity, NuValSeverity::Warning);
    assert_eq!(val.code.as_deref(), val.kind.code());
    let deprecation = nu::ShellError::DeprecationWarning {
        deprecation_type: "Command",
        suggestion: "use the other".into(),
        span: span(0, 4),
        help: None,
    };
    assert_eq!(NuValError::from(&deprecation).severity, NuValSeverity::Warning);
}

#[test]
fn an_outside_source_comes_with_its_text() {
    let error = nu::ShellError::OutsideSource {
        src: miette::NamedSource::new("script.nu", "let x = 1".to_string()),
        msg: "a message".into(),
        url: "https://example.org".into(),
        help: Some("some help".into()),
        labels: vec![miette::LabeledSpan::new(Some("here".into()), 4, 1)],
        inner: vec![],
    };
    let val = NuValError::from(&error);
    let source = val.source.expect("its own source");
    assert_eq!(source.name.as_deref(), Some("script.nu"));
    assert_eq!(source.text, "let x = 1");
    assert_eq!(val.url.as_deref(), Some("https://example.org"));
}

#[test]
fn each_family_holds_nushells_own_code() {
    let errors = [
        NuValError::from(&nu::ShellError::NotAList {
            dst_span: span(0, 1),
            src_span: span(1, 2),
        }),
        NuValError::from(&nu::ParseError::UnknownCommand(span(0, 3))),
        NuValError::from(&nu::CompileError::Garbage { span: span(0, 3) }),
        NuValError::from(&nu::ParseWarning::Deprecated {
            dep_type: "Flag".into(),
            label: "gone".into(),
            span: span(0, 1),
            help: None,
            report_mode: nu_protocol::ReportMode::EveryUse,
        }),
        NuValError::from(&nu::ConfigWarning::IncompatibleOptions {
            label: "both set",
            span: span(0, 1),
            help: "pick one",
        }),
    ];
    for val in errors {
        assert_eq!(val.code.as_deref(), val.kind.code(), "{:?}", val.kind);
    }
}

#[test]
fn an_error_survives_serde_bitcode_and_rkyv() {
    let error = nu::ShellError::Generic(
        nu_protocol::shell_error::generic::GenericError::new("A title", "the label", span(5, 9))
            .with_inner([nu::ShellError::DivisionByZero { span: span(6, 7) }]),
    );
    let val = NuValError::from(&error);
    let text = to_nuon(&val, NuonStyle::Raw).expect("serialize");
    assert_eq!(from_nuon::<NuValError>(&text).expect("deserialize"), val);
    let bytes = bitcode::encode(&val);
    assert_eq!(bitcode::decode::<NuValError>(&bytes).expect("decode"), val);
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&val).expect("archive");
    assert_eq!(
        rkyv::from_bytes::<NuValError, rkyv::rancor::Error>(&bytes).expect("unarchive"),
        val
    );
}

#[test]
fn back_in_nushell_an_error_is_a_labeled_error_with_its_parts() {
    let error = nu::ShellError::ExternalCommand {
        label: "the label".into(),
        help: "the help".into(),
        span: span(2, 8),
    };
    let val = NuValError::from(&error);
    let back = nu::ShellError::try_from(val.clone()).expect("back to nushell");
    let again = NuValError::from(&back);
    assert_eq!(again.kind, NuValErrorKind::Labeled);
    assert_eq!(again.msg, val.msg);
    assert_eq!(again.code, val.code);
    assert_eq!(again.labels, val.labels);
    assert_eq!(again.help, val.help);
}

#[test]
fn short_puts_an_error_on_one_line() {
    let parse = NuValError::from(&nu::ParseError::OutputMismatch(
        nu_protocol::Type::Int,
        "string".into(),
        span(10, 20),
    ));
    assert_eq!(
        parse.short(),
        "Command output doesn't match int: expected int, but command outputs string"
    );
    let shell = NuValError::from(&nu::ShellError::ExternalCommand {
        label: "Command `plugin` not found".into(),
        help: "`plugin` is neither a Nushell built-in or a known external command".into(),
        span: span(0, 6),
    });
    assert_eq!(
        shell.short(),
        "External command failed: Command `plugin` not found \
         (`plugin` is neither a Nushell built-in or a known external command)"
    );
    let bare = NuValError::from(&nu::ShellError::NushellFailed { msg: "x".into() });
    assert!(!bare.short().contains('\n'));
    let nested = NuValError::from(&nu::ShellError::Generic(
        nu_protocol::shell_error::generic::GenericError::new("Outer", "here", span(0, 1))
            .with_inner([nu::ShellError::DivisionByZero { span: span(2, 3) }]),
    ));
    assert_eq!(
        nested.short(),
        "Outer: here; Division by zero."
    );
}

#[test]
fn short_drops_a_label_its_message_repeats() {
    let not_found = NuValError::from(&nu::ShellError::Io(
        nu_protocol::shell_error::io::IoError::new(
            nu::IoKind::FileNotFound,
            span(0, 4),
            std::path::PathBuf::from("/x"),
        ),
    ));
    assert_eq!(not_found.short(), "File not found ('/x' does not exist)");
    let variable = NuValError::from(&nu::ParseError::VariableNotFound(
        nu_protocol::DidYouMean::new(&[], b""),
        span(0, 2),
    ));
    assert_eq!(variable.short(), "Variable not found.");
    let expected = NuValError::from(&nu::ParseError::Expected("block, closure or record", span(0, 2)));
    let text = expected.short();
    assert!(text.starts_with("Parse mismatch: expected block, closure or record. ("), "{text}");
    assert_eq!(text.matches("expected block, closure or record").count(), 1, "{text}");
    let extra = NuValError::from(&nu::ParseError::ExtraPositional("f".into(), span(0, 2)));
    assert_eq!(extra.short().matches("xtra positional argument").count(), 1, "{}", extra.short());
    let kept = NuValError::from(&nu::ShellError::Generic(
        nu_protocol::shell_error::generic::GenericError::new("Type mismatch", "expected int", span(0, 1)),
    ));
    assert_eq!(kept.short(), "Type mismatch: expected int");
}

#[test]
fn a_result_carries_its_error_both_ways() {
    let value = nu::Value::error(nu::ShellError::DivisionByZero { span: span(1, 2) }, SPAN);
    let result = NuValResult::try_from_nu(value).expect("converts");
    let Err(error) = &result else {
        panic!("an error result: {result:?}");
    };
    assert_eq!(
        error.kind,
        NuValErrorKind::Shell(ShellErrorKind::DivisionByZero)
    );
    let back = result.try_into_nu(SPAN).expect("back to nushell");
    let nu::Value::Error { error, .. } = back else {
        panic!("an error value");
    };
    assert_eq!(error.to_string(), "Division by zero.");
}

#[test]
fn a_reversed_span_is_refused_back_in_nushell() {
    let mut val = NuValError::from(&nu::ShellError::DivisionByZero { span: span(2, 8) });
    val.labels = vec![NuValErrorLabel {
        text: Some("here".into()),
        span: SpanData { start: 8, end: 2 },
        primary: true,
    }];
    assert!(matches!(
        nu::ShellError::try_from(val.clone()),
        Err(ConvertError::Span { start: 8, end: 2 })
    ));
    val.inner = vec![val.clone()];
    val.labels.clear();
    assert!(
        matches!(nu::ShellError::try_from(val), Err(ConvertError::Span { .. })),
        "an inner error's span is held too"
    );
}

#[test]
fn a_span_at_the_platform_end_saturates_rather_than_wrapping() {
    let label = NuValErrorLabel::from(miette::LabeledSpan::new(None, usize::MAX, 1));
    assert_eq!(label.span.start, wide(usize::MAX));
    assert_eq!(label.span.end, wide(usize::MAX).saturating_add(1));
    assert!(label.span.end >= label.span.start);
}
