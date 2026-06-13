//! Proc macros for tracing-wide.
//!
//! - `event!(Foo { a, b })` — constructs a `Message`, dispatches it to registered
//!   subscribers, then records it to tracing.
//! - `#[message(msg = "...")]` — marks a struct as an emittable message:
//!   implements `Message`/`MessageBehaviour`, fills the inherent consts,
//!   generates the recording, and asserts every field type implements `Field`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, format_ident, quote, quote_spanned};
use syn::{
    Attribute, Data, DeriveInput, Error, Expr, ExprLit, Fields, Ident, Lit, LitStr, Meta,
    MetaNameValue, Token, Type, parse::Parser, parse_macro_input, punctuated::Punctuated,
};

/// A `#[deprecated]` harvested from a `#[message]` struct or one of its
/// fields: `None` when absent, `Some("true")` when no reason is given,
/// otherwise the reason — `since` is not captured. rustc validates and
/// enforces the attribute itself (producers warn natively), so harvesting is
/// best-effort presence + reason. In tokens position it renders as the
/// `Option<&'static str>` a descriptor's `deprecated` field expects.
struct Deprecation(Option<String>);

/// `///` doc comments harvested from a struct or field. In tokens position it
/// renders as the `Option<&'static str>` a descriptor's `doc` field expects.
struct Docs(Option<String>);

/// One named field of a `#[message]` struct — everything the expansion needs
/// from it: the catalogue descriptor data, the recording ident, the ambient
/// join candidacy, and the type for the `Field` assertion. In tokens position
/// it renders as its catalogue `FieldDescriptor` literal.
struct MessageField {
    deprecated: Deprecation,
    doc: Docs,
    ident: Ident,
    meta: MetaPairs,
    ty: Type,
}

/// Arbitrary `key = <literal>` metadata pairs (message- or field-level). In
/// tokens position it renders as the `&[(&str, &str)]` slice literal a
/// descriptor's `meta` field expects.
#[derive(Default)]
struct MetaPairs(Vec<(String, String)>);

/// The `tags = [...]` routing labels, sorted + deduped + lowercased at
/// expansion so the emitted `&[&str]` is canonical and cheap to compare. In
/// tokens position it renders as the slice literal `&[ "a", "b" ]`.
#[derive(Default)]
struct Tags(Vec<String>);

impl Deprecation {
    /// The reason from the first `#[deprecated]` attribute, if any: handles
    /// the bare, `= "reason"`, and `(since = ..., note = ...)` forms; a form
    /// without a reason yields `"true"`.
    fn harvest(attrs: &[Attribute]) -> Self {
        let reason = |a: &Attribute| {
            let reason = match &a.meta {
                Meta::Path(_) => String::new(),
                Meta::NameValue(nv) => match &nv.value {
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }) => s.value(),
                    _ => String::new(),
                },
                Meta::List(_) => {
                    let mut note = String::new();
                    let _ = a.parse_nested_meta(|m| {
                        let s: LitStr = m.value()?.parse()?;
                        if m.path.is_ident("note") {
                            note = s.value();
                        }
                        Ok(())
                    });
                    note
                }
            };

            if reason.is_empty() {
                "true".to_string()
            } else {
                reason
            }
        };

        Deprecation(
            attrs
                .iter()
                .find(|a| a.path().is_ident("deprecated"))
                .map(reason),
        )
    }
}

impl Docs {
    /// Concatenate `#[doc = "..."]` (i.e. `///`) lines into one string, or `None`.
    fn harvest(attrs: &[Attribute]) -> Self {
        let lines: Vec<String> = attrs
            .iter()
            .filter(|a| a.path().is_ident("doc"))
            .filter_map(|a| match &a.meta {
                Meta::NameValue(nv) => match &nv.value {
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }) => Some(s.value().trim().to_string()),
                    _ => None,
                },
                _ => None,
            })
            .collect();

        Docs((!lines.is_empty()).then(|| lines.join("\n")))
    }
}

impl MessageField {
    /// The `T` if the field is syntactically `Option<T>` (also
    /// `option::Option<T>` etc.) — syntactic, like every derive that
    /// special-cases `Option`. Drives which fields the ambient join may fill.
    fn ambient_inner(&self) -> Option<&Type> {
        let Type::Path(tp) = &self.ty else {
            return None;
        };

        if tp.qself.is_some() {
            return None;
        }

        let seg = tp.path.segments.last()?;

        if seg.ident != "Option" {
            return None;
        }

        let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
            return None;
        };

        if args.args.len() != 1 {
            return None;
        }

        match args.args.first()? {
            syn::GenericArgument::Type(t) => Some(t),
            _ => None,
        }
    }

    fn parse(field: &syn::Field) -> syn::Result<Self> {
        let ident = field.ident.clone().expect("named field");

        // tracing's macros merge a field named `message` into the event's
        // message text, silently losing the key.
        if ident == "message" {
            return Err(Error::new_spanned(
                &ident,
                "a message field must not be named `message`; \
                 tracing reserves it for the event text",
            ));
        }

        let mut meta = MetaPairs::default();

        for a in &field.attrs {
            if a.path().is_ident("field") {
                let kvs =
                    a.parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)?;

                for kv in &kvs {
                    meta.push(kv)?;
                }
            }
        }

        Ok(MessageField {
            deprecated: Deprecation::harvest(&field.attrs),
            doc: Docs::harvest(&field.attrs),
            ident,
            meta,
            ty: field.ty.clone(),
        })
    }
}

impl MetaPairs {
    /// Accept one `key = <literal>` pair: the key must be an identifier, the
    /// value a literal (str/int/bool/float) — stringified, so the catalogue
    /// stores one uniform type.
    fn push(&mut self, kv: &MetaNameValue) -> syn::Result<()> {
        let Some(key) = kv.path.get_ident() else {
            return Err(Error::new_spanned(
                &kv.path,
                "metadata keys must be identifiers",
            ));
        };

        if self.0.iter().any(|(k, _)| key == k) {
            return Err(Error::new_spanned(
                key,
                format!("duplicate metadata key `{key}`"),
            ));
        }

        let value = match &kv.value {
            Expr::Lit(ExprLit {
                lit: Lit::Str(s), ..
            }) => s.value(),
            Expr::Lit(ExprLit {
                lit: Lit::Int(i), ..
            }) => i.base10_digits().to_string(),
            Expr::Lit(ExprLit {
                lit: Lit::Float(f), ..
            }) => f.base10_digits().to_string(),
            Expr::Lit(ExprLit {
                lit: Lit::Bool(b), ..
            }) => b.value.to_string(),
            other => {
                return Err(Error::new_spanned(
                    other,
                    "metadata values must be literals (str/int/bool/float)",
                ));
            }
        };

        self.0.push((key.to_string(), value));

        Ok(())
    }
}

impl Tags {
    /// Parse `["a", "b", …]`: each element a non-empty, lowercase string
    /// literal. Crate-prefix namespacing is deliberately *not* enforced —
    /// `Origin` already carries the originating crate.
    fn parse(value: &Expr) -> syn::Result<Self> {
        let Expr::Array(array) = value else {
            return Err(Error::new_spanned(
                value,
                "`tags` must be an array of string literals, e.g. `tags = [\"analytics\"]`",
            ));
        };

        let mut tags = Vec::with_capacity(array.elems.len());

        for elem in &array.elems {
            let Expr::Lit(ExprLit {
                lit: Lit::Str(s), ..
            }) = elem
            else {
                return Err(Error::new_spanned(
                    elem,
                    "each tag must be a string literal",
                ));
            };

            let tag = s.value();

            if tag.is_empty() {
                return Err(Error::new_spanned(s, "a tag must not be empty"));
            }

            if tag != tag.to_lowercase() {
                return Err(Error::new_spanned(
                    s,
                    format!("tags must be lowercase; use `{}`", tag.to_lowercase()),
                ));
            }

            tags.push(tag);
        }

        tags.sort_unstable();
        tags.dedup();

        Ok(Tags(tags))
    }
}

impl ToTokens for Deprecation {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.extend(option_str(&self.0));
    }
}

impl ToTokens for Docs {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        tokens.extend(option_str(&self.0));
    }
}

impl ToTokens for MessageField {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let MessageField {
            deprecated,
            doc,
            ident,
            meta,
            ty,
        } = self;

        tokens.extend(quote! {
            ::tracing_wide::catalogue::FieldDescriptor {
                deprecated: #deprecated,
                doc: #doc,
                meta: #meta,
                name: ::core::stringify!(#ident),
                r#type: ::core::stringify!(#ty),
            }
        });
    }
}

impl ToTokens for MetaPairs {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let keys = self.0.iter().map(|(k, _)| k);
        let vals = self.0.iter().map(|(_, v)| v);
        tokens.extend(quote! { &[ #( (#keys, #vals) ),* ] });
    }
}

impl ToTokens for Tags {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let tags = self.0.iter();

        tokens.extend(quote! { &[ #( #tags ),* ] });
    }
}

/// Function-like macro: construct a `Message` and record it.
///
/// `event!(Started { service, attempt })` evaluates the expression (typically a
/// struct literal whose field shorthand pulls in locals and arguments in scope),
/// checks it is a `Message`, and records it to the current tracing subscriber.
/// For spans, use `tracing::instrument` — tracing-wide deliberately ships no
/// span macro of its own.
#[proc_macro]
pub fn event(input: TokenStream) -> TokenStream {
    let expr = parse_macro_input!(input as Expr);

    quote! {{
        // `mut` is only exercised by the ambient join shim.
        #[allow(unused_mut)]
        let mut __tracing_wide_msg = #expr;
        ::tracing_wide::__private::MessageBehaviour::join_ambient(&mut __tracing_wide_msg);
        ::tracing_wide::__private::MessageBehaviour::emit(&__tracing_wide_msg);
    }}
    .into()
}

/// The `#[message]` expansion in `proc_macro2` terms, so it is unit-testable
/// and its output is pretty-printable.
fn expand_message(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let mut input: DeriveInput = syn::parse2(item)?;
    let name = input.ident.clone();

    // The generated impls and the catalogue's `TypeId::of` need one concrete
    // `'static` type; reject generics up front instead of leaking E0107s from
    // the expansion.
    if !input.generics.params.is_empty() {
        return Err(Error::new_spanned(
            &input.generics,
            "#[message] does not support generic parameters \
             (a message must be a concrete `'static` type)",
        ));
    }

    let mut msg: Option<String> = None;
    let mut level: Option<String> = None;
    let mut tags = Tags::default();
    let mut msg_meta = MetaPairs::default();

    let metas = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(attr)?;

    for m in metas {
        match m {
            Meta::Path(p) => {
                return Err(Error::new_spanned(
                    &p,
                    "expected `key = value`; `#[message]` takes no bare flags \
                     (serialization is enabled by `#[derive(Serialize)]`)",
                ));
            }
            Meta::NameValue(nv) if nv.path.is_ident("msg") => {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
                {
                    msg = Some(s.value());
                } else {
                    return Err(Error::new_spanned(
                        &nv.value,
                        "`msg` must be a string literal",
                    ));
                }
            }
            Meta::NameValue(nv) if nv.path.is_ident("level") => {
                let lvl = match &nv.value {
                    Expr::Path(p) if p.path.get_ident().is_some() => {
                        p.path.get_ident().unwrap().to_string()
                    }
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }) => s.value(),
                    _ => {
                        return Err(Error::new_spanned(
                            &nv.value,
                            "`level` must be one of trace/debug/info/warn/error",
                        ));
                    }
                };
                level = Some(lvl);
            }
            Meta::NameValue(nv) if nv.path.is_ident("tags") => {
                tags = Tags::parse(&nv.value)?;
            }
            Meta::NameValue(nv) => msg_meta.push(&nv)?,
            Meta::List(l) => {
                return Err(Error::new_spanned(
                    &l.path,
                    "expected `key = value` or a bare flag, not a list",
                ));
            }
        }
    }
    let msg = msg.unwrap_or_else(|| name.to_string());

    // tracing's macros treat the trailing literal as a format string; escape
    // braces so a `msg` containing `{`/`}` records verbatim instead of
    // triggering format-argument capture (rendering un-escapes them, so the
    // recorded text still matches `MSG`).
    let msg_record = msg.replace('{', "{{").replace('}', "}}");

    let msg_doc = Docs::harvest(&input.attrs);
    let msg_deprecation = Deprecation::harvest(&input.attrs);

    // The deprecation warning belongs at producer construction sites; exempt
    // the generated impls, which must keep naming the type.
    let allow_deprecated = if msg_deprecation.0.is_some() {
        quote! { #[allow(deprecated)] }
    } else {
        quote! {}
    };

    let (level_const, level_macro) = match level.as_deref().unwrap_or("info") {
        "trace" => (format_ident!("TRACE"), format_ident!("trace")),
        "debug" => (format_ident!("DEBUG"), format_ident!("debug")),
        "info" => (format_ident!("INFO"), format_ident!("info")),
        "warn" => (format_ident!("WARN"), format_ident!("warn")),
        "error" => (format_ident!("ERROR"), format_ident!("error")),
        other => {
            return Err(Error::new_spanned(
                &name,
                format!("unknown level `{other}` (expected trace/debug/info/warn/error)"),
            ));
        }
    };

    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => named
                .named
                .iter()
                .map(MessageField::parse)
                .collect::<syn::Result<Vec<_>>>()?,
            _ => {
                return Err(Error::new_spanned(
                    &name,
                    "#[message] requires named fields",
                ));
            }
        },
        _ => {
            return Err(Error::new_spanned(
                &name,
                "#[message] can only be applied to structs",
            ));
        }
    };

    // `#[field(...)]` is a helper attribute the compiler doesn't know; strip it
    // before re-emitting the struct.
    if let Data::Struct(s) = &mut input.data
        && let Fields::Named(named) = &mut s.fields
    {
        for f in &mut named.named {
            f.attrs.retain(|a| !a.path().is_ident("field"));
        }
    }

    let idents: Vec<&Ident> = fields.iter().map(|f| &f.ident).collect();

    let types: Vec<&Type> = fields.iter().map(|f| &f.ty).collect();

    let ambient = fields.iter().filter_map(|f| {
        f.ambient_inner().map(|inner| {
            let ident = &f.ident;
            quote! { (#ident, #inner) }
        })
    });

    // Location builtins at the struct's span resolve to the definition site,
    // filled by rustc while compiling the defining crate (the proc-macro can't
    // read spans on stable).
    let origin = quote_spanned! { name.span() =>
        ::tracing_wide::Origin {
            column: ::core::column!(),
            file: ::core::file!(),
            krate: ::core::env!("CARGO_PKG_NAME"),
            line: ::core::line!(),
            module: ::core::module_path!(),
        }
    };

    Ok(quote! {
        #input

        #allow_deprecated
        impl #name {
            /// Severity of this event type (drives the tracing macro below).
            pub const LEVEL: ::tracing_wide::__private::tracing::Level =
                ::tracing_wide::__private::tracing::Level::#level_const;

            /// Constant, static message text for this event type.
            pub const MSG: &'static str = #msg;

            /// Where this event type is defined — automatic provenance.
            pub const ORIGIN: ::tracing_wide::Origin = #origin;

            /// Sorted, deduped, lowercased routing tags for this event type.
            pub const TAGS: &'static [&'static str] = #tags;
        }

        #allow_deprecated
        impl ::tracing_wide::Message for #name {
            fn as_any(&self) -> &dyn ::core::any::Any { self }
            ::tracing_wide::__message_serialize_method! {}
            fn level(&self) -> ::tracing_wide::__private::tracing::Level { Self::LEVEL }
            fn msg(&self) -> &'static str { Self::MSG }
            fn origin(&self) -> &'static ::tracing_wide::Origin { &Self::ORIGIN }
            fn tags(&self) -> &'static [&'static str] { Self::TAGS }
        }

        #[doc(hidden)]
        #allow_deprecated
        impl ::tracing_wide::__private::Sealed for #name {}

        ::tracing_wide::__register_message! {
            ::tracing_wide::catalogue::MessageDescriptor {
                deprecated: #msg_deprecation,
                doc: #msg_doc,
                fields: &[ #( #fields ),* ],
                level: ::tracing_wide::__private::tracing::Level::#level_const,
                meta: #msg_meta,
                msg: #msg,
                origin: #name::ORIGIN,
                tags: #name::TAGS,
                type_id: ::core::any::TypeId::of::<#name>(),
            }
        }

        #[doc(hidden)]
        #allow_deprecated
        impl ::tracing_wide::__private::MessageBehaviour for #name {
            ::tracing_wide::__message_ambient_method! {
                #( #ambient ),*
            }

            // Deprecated fields keep recording (consumers mid-migration still
            // read them); only the producer's construction should warn.
            #[allow(deprecated)]
            fn record(&self) {
                ::tracing_wide::__private::tracing::#level_macro!( #( #idents = &self.#idents, )* #msg_record );
            }
        }

        const _: fn() = || {
            fn __tracing_wide_assert_field<T: ::tracing_wide::Field>() {}
            #( __tracing_wide_assert_field::<#types>(); )*
        };
    })
}

/// Attribute macro: mark a struct as a `Message`.
///
/// Implements the (pseudo-sealed) `Message`/`MessageBehaviour` traits, fills
/// the inherent `MSG` const from `#[message(msg = "...")]` (defaulting to the
/// struct name), generates the tracing recording, and asserts every field type
/// is a `Field`. The point is to keep the message static and put all variance
/// in the typed fields.
///
/// A struct-level `#[deprecated]` is honored like a field-level one: producers
/// get rustc's native warning, the note lands in the catalogue descriptor, and
/// the generated impls are exempted.
#[proc_macro_attribute]
pub fn message(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_message(attr.into(), item.into())
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Render an `Option<String>` as `Option<&'static str>` tokens — the shared
/// shape of the descriptors' `doc` and `deprecated` fields.
fn option_str(value: &Option<String>) -> TokenStream2 {
    match value {
        Some(s) => quote! { ::core::option::Option::Some(#s) },
        None => quote! { ::core::option::Option::None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn message_expansion_pretty_prints() {
        let attr = quote! { msg = "hi", level = warn, owner = "x" };
        let item = quote! {
            /// A demo event.
            struct Demo {
                a: usize,
                #[field(unit = "ms")]
                b: usize,
            }
        };
        let expanded = expand_message(attr, item).expect("expansion succeeds");
        let file = syn::parse2::<syn::File>(expanded).expect("output is valid Rust");
        let pretty = prettyplease::unparse(&file);

        assert!(pretty.contains("impl ::tracing_wide::Message for Demo"));
        assert!(pretty.contains("pub const MSG"));
        println!("{pretty}");
    }

    #[test]
    fn message_harvests_deprecation() {
        let pretty = |item: TokenStream2| {
            let expanded = expand_message(quote! { msg = "m" }, item).expect("expansion succeeds");
            prettyplease::unparse(&syn::parse2::<syn::File>(expanded).unwrap())
        };

        let noted = pretty(quote! { #[deprecated = "use `n`"] struct M { a: usize } });
        assert!(noted.contains(r#"Some("use `n`")"#), "{noted}");
        assert!(noted.contains("#[allow(deprecated)]\nimpl"), "{noted}");

        let bare = pretty(quote! { #[deprecated] struct M { a: usize } });
        assert!(bare.contains(r#"Some("true")"#), "{bare}");

        let meta =
            pretty(quote! { #[deprecated(since = "0.2", note = "gone")] struct M { a: usize } });
        assert!(meta.contains(r#"Some("gone")"#), "{meta}");

        let field = pretty(quote! { struct M { #[deprecated = "old"] a: usize } });
        assert!(field.contains(r#"Some("old")"#), "{field}");
        assert!(!field.contains("#[allow(deprecated)]\nimpl"), "{field}");

        // Macro-invocation contents print as raw tokens (spaced); normalize
        // whitespace before matching.
        let plain = pretty(quote! { struct M { a: usize } });
        let flat: String = plain.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(
            flat.contains("deprecated:::core::option::Option::None"),
            "{plain}"
        );
        assert!(!plain.contains("#[allow(deprecated)]\nimpl"), "{plain}");
    }

    #[test]
    fn message_rejects_non_literal_meta() {
        let err = expand_message(
            quote! { owner = some_path },
            quote! { struct M { a: usize } },
        )
        .unwrap_err();
        assert!(err.to_string().contains("must be literals"));
    }

    #[test]
    fn message_rejects_bare_flag() {
        let err =
            expand_message(quote! { serialize }, quote! { struct M { a: usize } }).unwrap_err();
        assert!(err.to_string().contains("takes no bare flags"));
    }

    #[test]
    fn message_rejects_non_struct() {
        let err = expand_message(quote! {}, quote! { enum E { A } }).unwrap_err();
        assert!(err.to_string().contains("can only be applied to structs"));
    }

    #[test]
    fn message_rejects_unnamed_fields() {
        let err = expand_message(quote! {}, quote! { struct T(usize); }).unwrap_err();
        assert!(err.to_string().contains("requires named fields"));
    }

    #[test]
    fn message_rejects_meta_list() {
        let err =
            expand_message(quote! { owner(x) }, quote! { struct M { a: usize } }).unwrap_err();
        assert!(err.to_string().contains("not a list"));
    }

    #[test]
    fn message_rejects_duplicate_meta_key() {
        let err = expand_message(
            quote! { owner = "a", owner = "b" },
            quote! { struct M { a: usize } },
        )
        .unwrap_err();
        assert!(err.to_string().contains("duplicate metadata key `owner`"));

        let err = expand_message(
            quote! {},
            quote! { struct M { #[field(unit = "ms", unit = "s")] a: usize } },
        )
        .unwrap_err();
        assert!(err.to_string().contains("duplicate metadata key `unit`"));
    }

    #[test]
    fn message_rejects_field_named_message() {
        let err = expand_message(quote! {}, quote! { struct M { message: usize } }).unwrap_err();
        assert!(err.to_string().contains("must not be named `message`"));
    }

    #[test]
    fn message_rejects_generic_params() {
        let cases = [
            quote! { struct M<T> { a: T } },
            quote! { struct M<'a> { a: &'a str } },
            quote! { struct M<const N: usize> { a: usize } },
        ];

        for item in cases {
            let err = expand_message(quote! {}, item).unwrap_err();
            assert!(err.to_string().contains("generic parameters"));
        }
    }

    #[test]
    fn message_escapes_braces_in_recorded_msg() {
        let expanded = expand_message(
            quote! { msg = "rate {limit} hit" },
            quote! { struct M { a: usize } },
        )
        .expect("expansion succeeds");
        let pretty = prettyplease::unparse(&syn::parse2::<syn::File>(expanded).unwrap());

        // The const keeps the text verbatim; only the tracing handoff (a
        // format string position) sees the escaped form.
        assert!(pretty.contains(r#""rate {limit} hit""#), "{pretty}");
        assert!(pretty.contains(r#""rate {{limit}} hit""#), "{pretty}");
    }

    #[test]
    fn message_rejects_non_ident_meta_key() {
        let err =
            expand_message(quote! { foo::bar = 1 }, quote! { struct M { a: usize } }).unwrap_err();
        assert!(err.to_string().contains("must be identifiers"));
    }

    #[test]
    fn message_rejects_non_string_msg() {
        let err = expand_message(quote! { msg = 5 }, quote! { struct M { a: usize } }).unwrap_err();
        assert!(err.to_string().contains("must be a string literal"));
    }

    #[test]
    fn message_rejects_unknown_level() {
        let err =
            expand_message(quote! { level = bogus }, quote! { struct M { a: usize } }).unwrap_err();
        assert!(err.to_string().contains("unknown level"));
    }

    #[test]
    fn message_sorts_and_dedups_tags() {
        let expanded = expand_message(
            quote! { msg = "m", tags = ["b", "a", "a"] },
            quote! { struct M { x: usize } },
        )
        .expect("expansion succeeds");
        let file = syn::parse2::<syn::File>(expanded).expect("output is valid Rust");
        let pretty = prettyplease::unparse(&file);
        assert!(pretty.contains("pub const TAGS"));
        assert!(
            pretty.contains(r#"["a", "b"]"#),
            "tags sorted+deduped: {pretty}"
        );
    }

    #[test]
    fn message_emits_origin_const() {
        let expanded =
            expand_message(quote! { msg = "m" }, quote! { struct M { x: usize } }).unwrap();
        let pretty = prettyplease::unparse(&syn::parse2::<syn::File>(expanded).unwrap());
        assert!(pretty.contains("pub const ORIGIN"));
        assert!(pretty.contains("CARGO_PKG_NAME"));
        assert!(pretty.contains("pub const TAGS"));
    }

    #[test]
    fn message_rejects_non_lowercase_tag() {
        let err = expand_message(
            quote! { tags = ["Security"] },
            quote! { struct M { a: usize } },
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("lowercase"), "{msg}");
        assert!(msg.contains("security"), "{msg}");
    }

    #[test]
    fn message_rejects_empty_tag() {
        let err =
            expand_message(quote! { tags = [""] }, quote! { struct M { a: usize } }).unwrap_err();
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn message_rejects_non_string_tag() {
        let err =
            expand_message(quote! { tags = [1] }, quote! { struct M { a: usize } }).unwrap_err();
        assert!(err.to_string().contains("string literal"));
    }

    #[test]
    fn message_rejects_non_array_tags() {
        let err = expand_message(
            quote! { tags = "security" },
            quote! { struct M { a: usize } },
        )
        .unwrap_err();
        assert!(err.to_string().contains("array of string literals"));
    }
}
