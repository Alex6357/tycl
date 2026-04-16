use anyhow::{Result, bail};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use tycl_parser::schema::{Schema, SchemaEntry};
use tycl_parser::types::{Type, TypeKind};
use tycl_parser::value::{TimeValue, Value};

pub fn generate_rust(schema: &Schema, root_name: &str) -> Result<TokenStream> {
    let mut generator = Generator::new();
    generator.generate(schema, root_name)
}

struct Generator {
    items: Vec<TokenStream>,
    emitted: HashSet<String>,
}

impl Generator {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            emitted: HashSet::new(),
        }
    }

    fn emit_name(&mut self, name: &str) -> Result<()> {
        if !self.emitted.insert(name.to_string()) {
            bail!("generated type name conflict: {}", name);
        }
        Ok(())
    }

    fn is_emitted(&self, name: &str) -> bool {
        self.emitted.contains(name)
    }

    fn generate(&mut self, schema: &Schema, root_name: &str) -> Result<TokenStream> {
        let root_ident = format_ident!("{}", sanitize_ident(root_name));
        self.emit_name(root_name)?;

        let mut field_decls = Vec::new();
        let mut field_convs = Vec::new();

        for (key, entry) in &schema.entries {
            if key == "$root-name" {
                continue;
            }
            let rust_ty = self.type_to_tokens(&entry.ty, key)?;
            let field_ident = format_ident!("{}", sanitize_ident(key));
            field_decls.push(quote! { pub #field_ident: #rust_ty, });

            let lookup_key = entry.target_name.as_ref().unwrap_or(key);
            let conv = self.root_field_conversion(entry, lookup_key)?;
            field_convs.push(quote! { #field_ident: #conv, });
        }

        let field_decls_stream = field_decls.into_iter().collect::<TokenStream>();
        let field_convs_stream = field_convs.into_iter().collect::<TokenStream>();

        let root_struct = quote! {
            pub struct #root_ident {
                #field_decls_stream
            }
        };

        let root_impl = quote! {
            impl #root_ident {
                fn try_from_document(doc: &tycl_parser::Document) -> Result<Self, tycl_parser::error::ValueAccessError> {
                    Ok(#root_ident {
                        #field_convs_stream
                    })
                }
            }
        };

        let items = &self.items;
        let items_stream = items.iter().cloned().collect::<TokenStream>();

        let output = quote! {
            #![allow(unused_imports)]
            use std::fmt;
            use tycl_parser::{Document, Value, TimeValue, error::ValueAccessError, error::ParseError};

            #[derive(Debug)]
            pub enum Error {
                Parse(ParseError),
                Access(ValueAccessError),
            }

            impl fmt::Display for Error {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    match self {
                        Error::Parse(e) => write!(f, "parse error: {}", e),
                        Error::Access(e) => write!(f, "access error: {}", e),
                    }
                }
            }

            impl std::error::Error for Error {}

            pub fn parse(source: &str) -> Result<#root_ident, Error> {
                let doc = tycl_parser::parse(source).map_err(Error::Parse)?;
                #root_ident::try_from_document(&doc).map_err(Error::Access)
            }

            pub fn parse_with_schema(source: &str, schema_source: &str) -> Result<#root_ident, Error> {
                let doc = tycl_parser::parse_with_schema_str(source, schema_source)
                    .map_err(Error::Parse)?;
                #root_ident::try_from_document(&doc).map_err(Error::Access)
            }

            #items_stream
            #root_struct
            #root_impl
        };

        Ok(output)
    }

    fn root_field_conversion(
        &mut self,
        entry: &SchemaEntry,
        lookup_key: &str,
    ) -> Result<TokenStream> {
        let has_default = !matches!(entry.default, Value::Null);
        let value_expr = if has_default {
            let default_tokens = value_to_tokens(&entry.default);
            quote! {
                doc.root.get(#lookup_key).cloned().unwrap_or_else(|| #default_tokens)
            }
        } else {
            quote! {
                doc.root.get(#lookup_key).cloned().ok_or_else(|| {
                    tycl_parser::error::ValueAccessError::MissingField {
                        field: #lookup_key.to_string(),
                    }
                })?
            }
        };
        self.conversion_expr(&entry.ty.kind, entry.ty.nullable, &value_expr, lookup_key)
    }

    fn type_to_tokens(&mut self, ty: &Type, name_hint: &str) -> Result<TokenStream> {
        let inner = match &ty.kind {
            TypeKind::Str(_) => quote!(String),
            TypeKind::Int(_) => quote!(i64),
            TypeKind::Float(_) => quote!(f64),
            TypeKind::Bool => quote!(bool),
            TypeKind::Any => quote!(::tycl_parser::Value),
            TypeKind::Time(_) => quote!(::tycl_parser::TimeValue),
            TypeKind::Map(inner) => {
                let inner_ty = self.type_to_tokens(&inner.node, name_hint)?;
                quote!(::std::collections::BTreeMap<String, #inner_ty>)
            }
            TypeKind::List(inner) => {
                let inner_ty = self.type_to_tokens(&inner.node, name_hint)?;
                quote!(Vec<#inner_ty>)
            }
            TypeKind::Tuple(elems, struct_name) => {
                if let Some(name) = struct_name {
                    let name = sanitize_ident(name);
                    let ident = format_ident!("{}", name);
                    if !self.is_emitted(&name) {
                        self.emit_name(&name)?;
                        let mut elem_types = Vec::new();
                        let mut field_builders = Vec::new();
                        for (i, e) in elems.iter().enumerate() {
                            let t =
                                self.type_to_tokens(&e.node, &format!("{}Elem{}", name_hint, i))?;
                            elem_types.push(t);
                            let conv = self.conversion_expr(
                                &e.node.kind,
                                e.node.nullable,
                                &quote!(items[#i]),
                                &format!("{}Elem{}", name_hint, i),
                            )?;
                            field_builders.push(quote!(#conv,));
                        }
                        let elem_types_stream = elem_types
                            .iter()
                            .map(|t| quote!(#t,))
                            .collect::<TokenStream>();
                        let field_builders_stream =
                            field_builders.into_iter().collect::<TokenStream>();
                        let item = quote! {
                            pub struct #ident(#elem_types_stream);

                            impl #ident {
                                fn try_from_value(value: &Value) -> Result<Self, ValueAccessError> {
                                    match value {
                                        Value::Tuple(items) | Value::List(items) => {
                                            let expected = #(elems.len());
                                            if items.len() != expected {
                                                return Err(ValueAccessError::TupleIndexOutOfBounds {
                                                    index: expected,
                                                    len: items.len(),
                                                });
                                            }
                                            Ok(#ident(#field_builders_stream))
                                        }
                                        _ => Err(ValueAccessError::TypeMismatch {
                                            expected: "tuple",
                                            found: value.type_name(),
                                        }),
                                    }
                                }
                            }
                        };
                        self.items.push(item);
                    }
                    quote!(#ident)
                } else {
                    let mut elem_types = Vec::new();
                    for (i, e) in elems.iter().enumerate() {
                        let t = self.type_to_tokens(&e.node, &format!("{}Elem{}", name_hint, i))?;
                        elem_types.push(t);
                    }
                    let elem_types_stream = elem_types
                        .iter()
                        .map(|t| quote!(#t,))
                        .collect::<TokenStream>();
                    quote!((#elem_types_stream))
                }
            }
            TypeKind::Record(fields, _open, struct_name) => {
                let name = match struct_name {
                    Some(n) => sanitize_ident(n),
                    None => bail!(
                        "Rust generator requires struct-name for record types: record(MyStruct)(...)"
                    ),
                };
                let ident = format_ident!("{}", name);
                if !self.is_emitted(&name) {
                    self.emit_name(&name)?;
                    let mut field_decls = Vec::new();
                    let mut field_convs = Vec::new();
                    for f in fields.iter() {
                        let field_ident = format_ident!("{}", sanitize_ident(&f.node.name.node));
                        let field_ty = self.type_to_tokens(&f.node.ty.node, &f.node.name.node)?;
                        field_decls.push(quote! { pub #field_ident: #field_ty, });

                        let lookup_key = f
                            .node
                            .target_name
                            .as_ref()
                            .map(|s| s.node.as_str())
                            .unwrap_or(f.node.name.node.as_str());
                        let has_default = f.node.default.is_some();
                        let value_expr = if has_default {
                            let default_tokens =
                                value_to_tokens(&f.node.default.as_ref().unwrap().node);
                            quote! {
                                map.get(#lookup_key).cloned().unwrap_or_else(|| #default_tokens)
                            }
                        } else {
                            quote! {
                                map.get(#lookup_key).cloned().ok_or_else(|| {
                                    ValueAccessError::MissingField {
                                        field: #lookup_key.to_string(),
                                    }
                                })?
                            }
                        };
                        let conv = self.conversion_expr(
                            &f.node.ty.node.kind,
                            f.node.ty.node.nullable,
                            &value_expr,
                            &f.node.name.node,
                        )?;
                        field_convs.push(quote! { #field_ident: #conv, });
                    }
                    let field_decls_stream = field_decls.into_iter().collect::<TokenStream>();
                    let field_convs_stream = field_convs.into_iter().collect::<TokenStream>();
                    let item = quote! {
                        pub struct #ident {
                            #field_decls_stream
                        }

                        impl #ident {
                            fn try_from_value(value: &Value) -> Result<Self, ValueAccessError> {
                                match value {
                                    Value::Record(map) | Value::Map(map) => {
                                        Ok(#ident {
                                            #field_convs_stream
                                        })
                                    }
                                    _ => Err(ValueAccessError::TypeMismatch {
                                        expected: "record",
                                        found: value.type_name(),
                                    }),
                                }
                            }
                        }
                    };
                    self.items.push(item);
                }
                quote!(#ident)
            }
            TypeKind::Enum(elems, struct_name) => {
                let name = match struct_name {
                    Some(n) => sanitize_ident(n),
                    None => to_pascal_case(name_hint),
                };
                let ident = format_ident!("{}", name);
                if !self.is_emitted(&name) {
                    self.emit_name(&name)?;
                    let mut variants = Vec::new();
                    let mut variant_matchers = Vec::new();
                    for e in elems.iter() {
                        let variant_ident = format_ident!(
                            "{}",
                            to_pascal_case(&sanitize_ident(&e.node.value.node))
                        );
                        variants.push(quote! { #variant_ident, });
                        let match_str = e
                            .node
                            .target_name
                            .as_ref()
                            .map(|s| s.node.as_str())
                            .unwrap_or(e.node.value.node.as_str());
                        variant_matchers.push(quote! { #match_str => Ok(#ident::#variant_ident), });
                    }
                    let variants_stream = variants.into_iter().collect::<TokenStream>();
                    let variant_matchers_stream =
                        variant_matchers.into_iter().collect::<TokenStream>();
                    let item = quote! {
                        pub enum #ident {
                            #variants_stream
                        }

                        impl #ident {
                            fn try_from_value(value: &Value) -> Result<Self, ValueAccessError> {
                                match value {
                                    Value::String(s) => match s.as_str() {
                                        #variant_matchers_stream
                                        _ => Err(ValueAccessError::TypeMismatch {
                                            expected: "enum variant",
                                            found: Box::leak(s.clone().into_boxed_str()),
                                        }),
                                    },
                                    _ => Err(ValueAccessError::TypeMismatch {
                                        expected: "string",
                                        found: value.type_name(),
                                    }),
                                }
                            }
                        }
                    };
                    self.items.push(item);
                }
                quote!(#ident)
            }
            TypeKind::Env { fallback, .. } => {
                if let Some(fb) = fallback {
                    self.type_to_tokens(&fb.node, name_hint)?
                } else {
                    quote!(String)
                }
            }
        };

        if ty.nullable {
            Ok(quote!(Option<#inner>))
        } else {
            Ok(inner)
        }
    }

    fn conversion_expr(
        &mut self,
        kind: &TypeKind,
        nullable: bool,
        value_expr: &TokenStream,
        name_hint: &str,
    ) -> Result<TokenStream> {
        if nullable {
            let inner = self.conversion_expr_inner(kind, &quote!(v), name_hint)?;
            return Ok(quote! {
                match #value_expr {
                    Value::Null => None,
                    v => Some(#inner),
                }
            });
        }
        self.conversion_expr_inner(kind, value_expr, name_hint)
    }

    fn conversion_expr_inner(
        &mut self,
        kind: &TypeKind,
        value_expr: &TokenStream,
        name_hint: &str,
    ) -> Result<TokenStream> {
        let expr = match kind {
            TypeKind::Str(_) => {
                quote! { #value_expr.as_string()?.unwrap().to_owned() }
            }
            TypeKind::Int(_) => {
                quote! { *#value_expr.as_integer()?.unwrap() }
            }
            TypeKind::Float(_) => {
                quote! { *#value_expr.as_float()?.unwrap() }
            }
            TypeKind::Bool => {
                quote! { *#value_expr.as_bool()?.unwrap() }
            }
            TypeKind::Any => {
                quote! { #value_expr.clone() }
            }
            TypeKind::Time(_) => {
                quote! { #value_expr.as_time()?.unwrap().clone() }
            }
            TypeKind::Map(inner) => {
                let inner_conv = self.conversion_expr(
                    &inner.node.kind,
                    inner.node.nullable,
                    &quote!(v),
                    name_hint,
                )?;
                quote! {
                    match #value_expr {
                        Value::Map(m) | Value::Record(m) => {
                            m.iter()
                                .map(|(k, v)| {
                                    let conv = #inner_conv;
                                    Ok((k.clone(), conv))
                                })
                                .collect::<Result<_, _>>()
                                .map_err(|e: ValueAccessError| e)?
                        }
                        _ => return Err(ValueAccessError::TypeMismatch {
                            expected: "map",
                            found: #value_expr.type_name(),
                        }),
                    }
                }
            }
            TypeKind::List(inner) => {
                let inner_conv = self.conversion_expr(
                    &inner.node.kind,
                    inner.node.nullable,
                    &quote!(v),
                    name_hint,
                )?;
                quote! {
                    match #value_expr {
                        Value::List(items) | Value::Tuple(items) => {
                            items
                                .iter()
                                .map(|v| {
                                    let conv = #inner_conv;
                                    Ok(conv)
                                })
                                .collect::<Result<Vec<_>, _>>()
                                .map_err(|e: ValueAccessError| e)?
                        }
                        _ => return Err(ValueAccessError::TypeMismatch {
                            expected: "list",
                            found: #value_expr.type_name(),
                        }),
                    }
                }
            }
            TypeKind::Tuple(elems, struct_name) => {
                if struct_name.is_some() {
                    let type_name = self.type_to_tokens(
                        &Type {
                            kind: kind.clone(),
                            nullable: false,
                        },
                        name_hint,
                    )?;
                    quote! { #type_name::try_from_value(&#value_expr)? }
                } else {
                    let mut field_builders = Vec::new();
                    for (i, e) in elems.iter().enumerate() {
                        let conv = self.conversion_expr(
                            &e.node.kind,
                            e.node.nullable,
                            &quote!(items[#i]),
                            &format!("{}Elem{}", name_hint, i),
                        )?;
                        field_builders.push(quote!(#conv));
                    }
                    let field_builders_stream = field_builders
                        .iter()
                        .map(|t| quote!(#t,))
                        .collect::<TokenStream>();
                    quote! {
                        match #value_expr {
                            Value::Tuple(items) | Value::List(items) => {
                                let expected = #(elems.len());
                                if items.len() != expected {
                                    return Err(ValueAccessError::TupleIndexOutOfBounds {
                                        index: expected,
                                        len: items.len(),
                                    });
                                }
                                (#field_builders_stream)
                            }
                            _ => return Err(ValueAccessError::TypeMismatch {
                                expected: "tuple",
                                found: #value_expr.type_name(),
                            }),
                        }
                    }
                }
            }
            TypeKind::Record(_, _, _) | TypeKind::Enum(_, _) => {
                let type_name = self.type_to_tokens(
                    &Type {
                        kind: kind.clone(),
                        nullable: false,
                    },
                    name_hint,
                )?;
                quote! { #type_name::try_from_value(&#value_expr)? }
            }
            TypeKind::Env { fallback, .. } => {
                if let Some(fb) = fallback {
                    self.conversion_expr(&fb.node.kind, fb.node.nullable, value_expr, name_hint)?
                } else {
                    quote! { #value_expr.as_string()?.unwrap().to_owned() }
                }
            }
        };
        Ok(expr)
    }
}

fn sanitize_ident(s: &str) -> String {
    let mut s = s.replace('-', "_");
    if s.chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        s.insert(0, '_');
    }
    const KEYWORDS: &[&str] = &[
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while",
    ];
    if KEYWORDS.contains(&s.as_str()) {
        s.push('_');
    }
    s
}

fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| c == '-' || c == '_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn value_to_tokens(value: &Value) -> TokenStream {
    match value {
        Value::Null => quote!(tycl_parser::Value::Null),
        Value::Bool(v) => quote!(tycl_parser::Value::Bool(#v)),
        Value::Integer(v) => quote!(tycl_parser::Value::Integer(#v)),
        Value::Float(v) => quote!(tycl_parser::Value::Float(#v)),
        Value::String(v) => quote!(tycl_parser::Value::String(#v.to_string())),
        Value::Time(tv) => match tv {
            TimeValue::LocalDate(s) => {
                quote!(tycl_parser::Value::Time(tycl_parser::TimeValue::LocalDate(#s.to_string())))
            }
            TimeValue::LocalTime(s) => {
                quote!(tycl_parser::Value::Time(tycl_parser::TimeValue::LocalTime(#s.to_string())))
            }
            TimeValue::LocalDateTime(s) => {
                quote!(tycl_parser::Value::Time(tycl_parser::TimeValue::LocalDateTime(#s.to_string())))
            }
            TimeValue::OffsetDateTime(s) => {
                quote!(tycl_parser::Value::Time(tycl_parser::TimeValue::OffsetDateTime(#s.to_string())))
            }
        },
        Value::List(items) => {
            let elems = items
                .iter()
                .map(value_to_tokens)
                .map(|t| quote!(#t,))
                .collect::<TokenStream>();
            quote!(tycl_parser::Value::List(vec![#elems]))
        }
        Value::Tuple(items) => {
            let elems = items
                .iter()
                .map(value_to_tokens)
                .map(|t| quote!(#t,))
                .collect::<TokenStream>();
            quote!(tycl_parser::Value::Tuple(vec![#elems]))
        }
        Value::Map(map) => {
            let entries = map
                .iter()
                .map(|(k, v)| {
                    let vt = value_to_tokens(v);
                    quote! { m.insert(#k.to_string(), #vt); }
                })
                .collect::<TokenStream>();
            quote! {
                {
                    let mut m = ::std::collections::BTreeMap::new();
                    #entries
                    tycl_parser::Value::Map(m)
                }
            }
        }
        Value::Record(map) => {
            let entries = map
                .iter()
                .map(|(k, v)| {
                    let vt = value_to_tokens(v);
                    quote! { m.insert(#k.to_string(), #vt); }
                })
                .collect::<TokenStream>();
            quote! {
                {
                    let mut m = ::std::collections::BTreeMap::new();
                    #entries
                    tycl_parser::Value::Record(m)
                }
            }
        }
    }
}
