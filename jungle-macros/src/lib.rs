use proc_macro::TokenStream;
use quote::quote;
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
        return syn::Error::new_spanned(
            args,
            "this attribute does not accept arguments",
        )
        .into_compile_error()
        .into();
    }

    let item = parse_macro_input!(input as syn::Item);
    let errors = detect_item(&item);
    let mut out = quote!(#item);
    if let Some(err_tokens) = errors {
        out.extend(err_tokens);
    }
    out.into()
}

fn detect_item(item: &syn::Item) -> Option<proc_macro2::TokenStream> {
    let mut errors = Vec::<syn::Error>::new();

    match item {
        syn::Item::Impl(item_impl) => {
            let methods = inspectable_impl_methods(item_impl);
            for method in methods {
                let mut detector = NondeterminismDetector::default();
                detector.visit_block(&method.block);
                for finding in detector.findings {
                    errors.push(syn::Error::new(
                        finding.span,
                        format!(
                            "jungle detect: suspected nondeterministic call `{}` in `{}`; {}",
                            finding.call, method.sig.ident, finding.reason
                        ),
                    ));
                }
            }
        }
        syn::Item::Fn(item_fn) => {
            let mut detector = NondeterminismDetector::default();
            detector.visit_block(&item_fn.block);
            for finding in detector.findings {
                errors.push(syn::Error::new(
                    finding.span,
                    format!(
                        "jungle detect: suspected nondeterministic call `{}` in `{}`; {}",
                        finding.call, item_fn.sig.ident, finding.reason
                    ),
                ));
            }
        }
        _ => {
            errors.push(syn::Error::new(
                item.span(),
                "#[detect] supports impl blocks and functions only",
            ));
        }
    }

    if errors.is_empty() {
        None
    } else {
        let mut combined = proc_macro2::TokenStream::new();
        for err in errors {
            combined.extend(err.to_compile_error());
        }
        Some(combined)
    }
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
            Some("Act") => matches!(method_name.as_str(), "emit" | "absorb"),
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
    findings: Vec<Finding>,
}

struct Finding {
    span: proc_macro2::Span,
    call: String,
    reason: &'static str,
}

impl NondeterminismDetector {
    fn check_path(&mut self, span: proc_macro2::Span, path: &syn::Path) {
        let normalized = normalize_path(path);
        for rule in path_rules() {
            if normalized == rule.path || normalized.starts_with(rule.prefix) {
                self.findings.push(Finding {
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
                self.findings.push(Finding {
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
            path: "std::env::current_dir",
            prefix: "std::env::current_dir::",
            reason: "ambient process state can differ across workers",
        },
        PathRule {
            path: "std::thread::spawn",
            prefix: "std::thread::spawn::",
            reason: "thread scheduling introduces nondeterministic outcomes",
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_flags_nondeterministic_emit_calls() {
        let item: syn::Item = syn::parse_quote! {
            impl Act<MyAnimal> for MyStep {
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
        let errors = detect_item(&item).expect("detector should flag now()");
        let text = errors.to_string();
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
        assert!(detect_item(&item).is_none());
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
        let errors = detect_item(&item).expect("detector should flag choose()");
        let text = errors.to_string();
        assert!(text.contains("SystemTime::now"));
        assert!(text.contains("choose"));
    }
}
