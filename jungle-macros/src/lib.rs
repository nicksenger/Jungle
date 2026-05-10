use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{parse_macro_input, parse_quote, DeriveInput, Path};

#[proc_macro]
pub fn noop(input: TokenStream) -> TokenStream {
    input
}

fn derive_with_properties(input: TokenStream, properties: &[Path]) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    input
        .attrs
        .push(parse_quote!(#[inception(properties = [#(#properties),*])]));
    inception_derive_gen::State::gen(quote!(#input)).into()
}

#[proc_macro_derive(Journey)]
pub fn derive_journey(input: TokenStream) -> TokenStream {
    derive_with_properties(
        input,
        &[
            parse_quote!(jungle_types::JungleFlow),
            parse_quote!(jungle_types::JungleDynFlow),
            parse_quote!(jungle_types::JungleTraverseFlow),
            parse_quote!(jungle_types::JungleReplaceFlow),
        ],
    )
}

#[proc_macro_derive(Flow)]
pub fn derive_flow(input: TokenStream) -> TokenStream {
    derive_with_properties(
        input,
        &[
            parse_quote!(jungle_types::JungleFlow),
            parse_quote!(jungle_types::JungleTraverseFlow),
            parse_quote!(jungle_types::JungleReplaceFlow),
        ],
    )
}

#[proc_macro_derive(Animals)]
pub fn derive_animals(input: TokenStream) -> TokenStream {
    derive_with_properties(
        input,
        &[
            parse_quote!(jungle_types::Ident),
            parse_quote!(jungle_types::JungleAnimals),
        ],
    )
}

#[proc_macro_derive(Actions)]
pub fn derive_actions(input: TokenStream) -> TokenStream {
    derive_with_properties(
        input,
        &[
            parse_quote!(jungle_types::Ident),
            parse_quote!(jungle_types::JungleActions),
        ],
    )
}

#[proc_macro_derive(Optic)]
pub fn derive_optic(input: TokenStream) -> TokenStream {
    derive_with_properties(input, &[parse_quote!(jungle_types::JungleOptic)])
}

fn expand_with_properties(
    attr: TokenStream,
    input: TokenStream,
    properties: &[Path],
) -> TokenStream {
    let args = proc_macro2::TokenStream::from(attr);
    if !args.is_empty() {
        return syn::Error::new_spanned(args, "this attribute does not accept arguments")
            .into_compile_error()
            .into();
    }

    let item = parse_macro_input!(input as syn::Item);
    quote! {
        #[inception::inception_derive(properties = [#(#properties),*])]
        #item
    }
    .into()
}

#[proc_macro_attribute]
pub fn journey(attr: TokenStream, input: TokenStream) -> TokenStream {
    expand_with_properties(
        attr,
        input,
        &[
            parse_quote!(jungle_types::JungleFlow),
            parse_quote!(jungle_types::JungleDynFlow),
            parse_quote!(jungle_types::JungleTraverseFlow),
            parse_quote!(jungle_types::JungleReplaceFlow),
        ],
    )
}

#[proc_macro_attribute]
pub fn flow(attr: TokenStream, input: TokenStream) -> TokenStream {
    expand_with_properties(
        attr,
        input,
        &[
            parse_quote!(jungle_types::JungleFlow),
            parse_quote!(jungle_types::JungleTraverseFlow),
            parse_quote!(jungle_types::JungleReplaceFlow),
        ],
    )
}

#[proc_macro_attribute]
pub fn animals(attr: TokenStream, input: TokenStream) -> TokenStream {
    expand_with_properties(
        attr,
        input,
        &[
            parse_quote!(jungle_types::Ident),
            parse_quote!(jungle_types::JungleAnimals),
        ],
    )
}

#[proc_macro_attribute]
pub fn actions(attr: TokenStream, input: TokenStream) -> TokenStream {
    expand_with_properties(
        attr,
        input,
        &[
            parse_quote!(jungle_types::Ident),
            parse_quote!(jungle_types::JungleActions),
        ],
    )
}

#[proc_macro_attribute]
pub fn detect(attr: TokenStream, input: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(attr);
    if !args.is_empty() {
        return syn::Error::new_spanned(args, "this attribute does not accept arguments")
            .into_compile_error()
            .into();
    }

    let item = parse_macro_input!(input as syn::Item);
    let mode = detect_mode();
    let findings = match collect_findings(&item) {
        Ok(findings) => findings,
        Err(err) => return err.into_compile_error().into(),
    };

    let mut out = quote!(#item);
    if let Some(diagnostics) = render_diagnostics(&findings, mode) {
        out.extend(diagnostics);
    }
    out.into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DetectMode {
    Disabled,
    Warn,
    Error,
}

fn detect_mode() -> DetectMode {
    if cfg!(feature = "detect-error") {
        DetectMode::Error
    } else if cfg!(feature = "detect-warn") {
        DetectMode::Warn
    } else {
        DetectMode::Disabled
    }
}

fn render_diagnostics(findings: &[Finding], mode: DetectMode) -> Option<proc_macro2::TokenStream> {
    if findings.is_empty() || mode == DetectMode::Disabled {
        return None;
    }

    let mut out = proc_macro2::TokenStream::new();
    match mode {
        DetectMode::Error => {
            for finding in findings {
                out.extend(
                    syn::Error::new(finding.span, finding.message.clone()).to_compile_error(),
                );
            }
        }
        DetectMode::Warn => {
            for (idx, finding) in findings.iter().enumerate() {
                let note = &finding.message;
                let warning_const = format_ident!("_JUNGLE_DETECT_WARNING_{idx}");
                out.extend(quote_spanned! {finding.span=>
                    #[allow(dead_code)]
                    const _: () = {
                        #[deprecated(note = #note)]
                        const #warning_const: () = ();
                        let _ = #warning_const;
                    };
                });
            }
        }
        DetectMode::Disabled => {}
    }

    Some(out)
}

fn collect_findings(item: &syn::Item) -> Result<Vec<Finding>, syn::Error> {
    let mut findings = Vec::<Finding>::new();

    match item {
        syn::Item::Impl(item_impl) => {
            let methods = inspectable_impl_methods(item_impl);
            for method in methods {
                let mut detector = NondeterminismDetector::default();
                detector.visit_block(&method.block);
                findings.extend(detector.findings.into_iter().map(|finding| Finding {
                    span: finding.span,
                    message: format!(
                        "jungle detect: suspected nondeterministic call `{}` in `{}`; {}",
                        finding.call, method.sig.ident, finding.reason
                    ),
                }));
            }
        }
        syn::Item::Fn(item_fn) => {
            let mut detector = NondeterminismDetector::default();
            detector.visit_block(&item_fn.block);
            findings.extend(detector.findings.into_iter().map(|finding| Finding {
                span: finding.span,
                message: format!(
                    "jungle detect: suspected nondeterministic call `{}` in `{}`; {}",
                    finding.call, item_fn.sig.ident, finding.reason
                ),
            }));
        }
        _ => {
            return Err(syn::Error::new(
                item.span(),
                "#[detect] supports impl blocks and functions only",
            ));
        }
    }

    Ok(findings)
}

fn inspectable_impl_methods(item_impl: &syn::ItemImpl) -> Vec<&syn::ImplItemFn> {
    let mut methods = Vec::new();
    let trait_name = item_impl
        .trait_
        .as_ref()
        .and_then(|(_, path, _)| path.segments.last())
        .map(|segment| segment.ident.to_string());

    for item in &item_impl.items {
        let syn::ImplItem::Fn(method) = item else {
            continue;
        };
        let method_name = method.sig.ident.to_string();
        let inspect = match trait_name.as_deref() {
            Some("Pulse") => matches!(method_name.as_str(), "emit" | "absorb"),
            Some("Condition") => method_name == "choose",
            Some("LoopCondition") => method_name == "should_continue",
            _ => matches!(
                method_name.as_str(),
                "emit" | "absorb" | "choose" | "should_continue"
            ),
        };
        if inspect {
            methods.push(method);
        }
    }

    methods
}

#[derive(Default)]
struct NondeterminismDetector {
    findings: Vec<DetectedCall>,
}

struct Finding {
    span: proc_macro2::Span,
    message: String,
}

struct DetectedCall {
    span: proc_macro2::Span,
    call: String,
    reason: &'static str,
}

impl NondeterminismDetector {
    fn check_path(&mut self, span: proc_macro2::Span, path: &syn::Path) {
        let normalized = normalize_path(path);
        for rule in path_rules() {
            if normalized == rule.path || normalized.starts_with(rule.prefix) {
                self.findings.push(DetectedCall {
                    span,
                    call: normalized,
                    reason: rule.reason,
                });
                return;
            }
        }
    }

    fn check_method(&mut self, span: proc_macro2::Span, method: &str) {
        for rule in method_rules() {
            if method == rule.method {
                self.findings.push(DetectedCall {
                    span,
                    call: method.to_string(),
                    reason: rule.reason,
                });
                return;
            }
        }
    }
}

impl<'ast> Visit<'ast> for NondeterminismDetector {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path_expr) = &*node.func {
            self.check_path(path_expr.path.span(), &path_expr.path);
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        self.check_method(node.method.span(), &node.method.to_string());
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.check_path(mac.path.span(), &mac.path);
        syn::visit::visit_macro(self, mac);
    }
}

fn normalize_path(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

struct PathRule {
    path: &'static str,
    prefix: &'static str,
    reason: &'static str,
}

fn path_rules() -> &'static [PathRule] {
    &[
        PathRule {
            path: "std::time::SystemTime::now",
            prefix: "std::time::SystemTime::now::",
            reason: "wall-clock time is not replay-deterministic",
        },
        PathRule {
            path: "std::time::Instant::now",
            prefix: "std::time::Instant::now::",
            reason: "monotonic clock reads are not replay-deterministic",
        },
        PathRule {
            path: "chrono::Utc::now",
            prefix: "chrono::Utc::now::",
            reason: "wall-clock time is not replay-deterministic",
        },
        PathRule {
            path: "chrono::Local::now",
            prefix: "chrono::Local::now::",
            reason: "local clock/timezone reads are not replay-deterministic",
        },
        PathRule {
            path: "rand::thread_rng",
            prefix: "rand::thread_rng::",
            reason: "ambient randomness is not replay-deterministic",
        },
        PathRule {
            path: "rand::random",
            prefix: "rand::random::",
            reason: "ambient randomness is not replay-deterministic",
        },
        PathRule {
            path: "getrandom::getrandom",
            prefix: "getrandom::getrandom::",
            reason: "ambient randomness is not replay-deterministic",
        },
        PathRule {
            path: "uuid::Uuid::new_v4",
            prefix: "uuid::Uuid::new_v4::",
            reason: "random UUID generation is not replay-deterministic",
        },
        PathRule {
            path: "std::env::var",
            prefix: "std::env::var::",
            reason: "environment access can differ across workers",
        },
        PathRule {
            path: "std::env::var_os",
            prefix: "std::env::var_os::",
            reason: "environment access can differ across workers",
        },
        PathRule {
            path: "std::env::vars",
            prefix: "std::env::vars::",
            reason: "environment access can differ across workers",
        },
        PathRule {
            path: "std::env::vars_os",
            prefix: "std::env::vars_os::",
            reason: "environment access can differ across workers",
        },
        PathRule {
            path: "std::env::temp_dir",
            prefix: "std::env::temp_dir::",
            reason: "ambient process state can differ across workers",
        },
        PathRule {
            path: "std::env::current_dir",
            prefix: "std::env::current_dir::",
            reason: "ambient process state can differ across workers",
        },
        PathRule {
            path: "std::process::id",
            prefix: "std::process::id::",
            reason: "process identifiers vary across workers",
        },
        PathRule {
            path: "std::process::Command::new",
            prefix: "std::process::Command::new::",
            reason: "process execution depends on ambient host state",
        },
        PathRule {
            path: "std::fs::read",
            prefix: "std::fs::read::",
            reason: "filesystem reads are external nondeterministic inputs",
        },
        PathRule {
            path: "std::fs::read_to_string",
            prefix: "std::fs::read_to_string::",
            reason: "filesystem reads are external nondeterministic inputs",
        },
        PathRule {
            path: "std::fs::metadata",
            prefix: "std::fs::metadata::",
            reason: "filesystem metadata is external nondeterministic input",
        },
        PathRule {
            path: "std::fs::canonicalize",
            prefix: "std::fs::canonicalize::",
            reason: "filesystem state can differ across workers",
        },
        PathRule {
            path: "std::net::TcpStream::connect",
            prefix: "std::net::TcpStream::connect::",
            reason: "network I/O is not replay-deterministic",
        },
        PathRule {
            path: "std::net::UdpSocket::bind",
            prefix: "std::net::UdpSocket::bind::",
            reason: "network and ephemeral-port assignment are nondeterministic",
        },
        PathRule {
            path: "std::net::ToSocketAddrs::to_socket_addrs",
            prefix: "std::net::ToSocketAddrs::to_socket_addrs::",
            reason: "DNS/address resolution is external nondeterministic input",
        },
        PathRule {
            path: "tokio::time::Instant::now",
            prefix: "tokio::time::Instant::now::",
            reason: "clock reads are not replay-deterministic",
        },
        PathRule {
            path: "tokio::time::sleep",
            prefix: "tokio::time::sleep::",
            reason: "ambient sleep should be expressed via framework sleep action",
        },
        PathRule {
            path: "tokio::task::spawn",
            prefix: "tokio::task::spawn::",
            reason: "task scheduling introduces nondeterministic outcomes",
        },
        PathRule {
            path: "std::thread::spawn",
            prefix: "std::thread::spawn::",
            reason: "thread scheduling introduces nondeterministic outcomes",
        },
        PathRule {
            path: "std::thread::sleep",
            prefix: "std::thread::sleep::",
            reason: "ambient sleep should be expressed via framework sleep action",
        },
        PathRule {
            path: "std::thread::yield_now",
            prefix: "std::thread::yield_now::",
            reason: "scheduler interaction can introduce nondeterministic outcomes",
        },
        PathRule {
            path: "tokio::spawn",
            prefix: "tokio::spawn::",
            reason: "task scheduling introduces nondeterministic outcomes",
        },
    ]
}

struct MethodRule {
    method: &'static str,
    reason: &'static str,
}

fn method_rules() -> &'static [MethodRule] {
    &[
        MethodRule {
            method: "gen",
            reason: "RNG generation is not replay-deterministic",
        },
        MethodRule {
            method: "gen_range",
            reason: "RNG generation is not replay-deterministic",
        },
        MethodRule {
            method: "gen_bool",
            reason: "RNG generation is not replay-deterministic",
        },
        MethodRule {
            method: "gen_ratio",
            reason: "RNG generation is not replay-deterministic",
        },
        MethodRule {
            method: "next_u32",
            reason: "RNG generation is not replay-deterministic",
        },
        MethodRule {
            method: "next_u64",
            reason: "RNG generation is not replay-deterministic",
        },
        MethodRule {
            method: "next_u128",
            reason: "RNG generation is not replay-deterministic",
        },
        MethodRule {
            method: "sample",
            reason: "RNG generation is not replay-deterministic",
        },
        MethodRule {
            method: "shuffle",
            reason: "RNG generation is not replay-deterministic",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_flags_nondeterministic_emit_calls() {
        let item: syn::Item = syn::parse_quote! {
            impl Pulse<MyAnimal> for MyStep {
                type Action = MyAction;
                type Aspect = Identity;
                type In = ();
                type Out = ();

                fn emit(_state: &State, _input: Self::In) -> () {
                    let _ = chrono::Utc::now();
                }

                fn absorb(_state: &mut State, _output: ActionCompletion<Self::Action>) -> () {}
            }
        };
        let findings = collect_findings(&item).expect("item should parse");
        let text = findings
            .iter()
            .map(|finding| finding.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("chrono::Utc::now"));
        assert!(text.contains("emit"));
    }

    #[test]
    fn detect_skips_irrelevant_impl_methods() {
        let item: syn::Item = syn::parse_quote! {
            impl NotAFlowTrait for MyType {
                fn helper() {
                    let _ = chrono::Utc::now();
                }
            }
        };
        let findings = collect_findings(&item).expect("item should parse");
        assert!(findings.is_empty());
    }

    #[test]
    fn detect_flags_condition_choose() {
        let item: syn::Item = syn::parse_quote! {
            impl Condition<(State, ())> for MyCondition {
                fn choose(_input: &(State, ())) -> bool {
                    let _ = std::time::SystemTime::now();
                    true
                }
            }
        };
        let findings = collect_findings(&item).expect("item should parse");
        let text = findings
            .iter()
            .map(|finding| finding.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("SystemTime::now"));
        assert!(text.contains("choose"));
    }

    #[test]
    fn detect_covers_sleep_and_spawn_patterns() {
        let item: syn::Item = syn::parse_quote! {
            impl Pulse<MyAnimal> for MyStep {
                type Action = MyAction;
                type Aspect = Identity;
                type In = ();
                type Out = ();

                fn emit(_state: &State, _input: Self::In) -> () {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    tokio::task::spawn(async move {});
                }

                fn absorb(_state: &mut State, _output: ActionCompletion<Self::Action>) -> () {}
            }
        };
        let findings = collect_findings(&item).expect("item should parse");
        let text = findings
            .iter()
            .map(|finding| finding.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("std::thread::sleep"));
        assert!(text.contains("tokio::task::spawn"));
    }

    #[test]
    fn diagnostics_disabled_mode_emits_nothing() {
        let findings = vec![Finding {
            span: proc_macro2::Span::call_site(),
            message: "x".to_string(),
        }];
        let rendered = render_diagnostics(&findings, DetectMode::Disabled);
        assert!(rendered.is_none());
    }

    #[test]
    fn diagnostics_warn_mode_emits_tokens() {
        let findings = vec![Finding {
            span: proc_macro2::Span::call_site(),
            message: "x".to_string(),
        }];
        let rendered = render_diagnostics(&findings, DetectMode::Warn)
            .expect("warn mode should produce diagnostics");
        assert!(rendered.to_string().contains("deprecated"));
    }
}
