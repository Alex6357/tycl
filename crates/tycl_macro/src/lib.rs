use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, Lit, Type};

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        let path = &type_path.path;
        if path.segments.len() == 1 && path.segments[0].ident == "Option" {
            return true;
        }
        if let Some(segment) = path.segments.last() {
            if segment.ident == "Option" {
                return true;
            }
        }
    }
    false
}

fn get_tycl_rename(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("tycl") {
            continue;
        }
        let mut rename = None;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value = meta.value()?;
                let lit: Lit = value.parse()?;
                if let Lit::Str(s) = lit {
                    rename = Some(s.value());
                }
            }
            Ok(())
        });
        if rename.is_some() {
            return rename;
        }
    }
    None
}

#[proc_macro_derive(TryFromValue, attributes(tycl))]
pub fn derive_try_from_value(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let field_builders = fields.named.iter().map(|f| {
                    let field_name = &f.ident;
                    let rename = get_tycl_rename(&f.attrs);
                    let field_str = rename.unwrap_or_else(|| field_name.as_ref().unwrap().to_string());
                    let field_ty = &f.ty;
                    let is_option = is_option_type(field_ty);

                    if is_option {
                        quote! {
                            #field_name: {
                                match map.get(#field_str) {
                                    None | Some(::tycl_parser::value::Value::Null) => None,
                                    Some(v) => {
                                        <#field_ty as ::tycl_parser::value::TryFromValue>::try_from_value(v.clone())?
                                    }
                                }
                            },
                        }
                    } else {
                        quote! {
                            #field_name: {
                                let v = map.get(#field_str).cloned().ok_or_else(|| {
                                    ::tycl_parser::error::ValueAccessError::MissingField {
                                        field: #field_str.to_string(),
                                    }
                                })?;
                                <#field_ty as ::tycl_parser::value::TryFromValue>::try_from_value(v)?
                            },
                        }
                    }
                });

                quote! {
                    let map = match value {
                        ::tycl_parser::value::Value::Record(map) => map,
                        _ => {
                            return Err(::tycl_parser::error::ValueAccessError::TypeMismatch {
                                expected: "record",
                                found: value.type_name(),
                            });
                        }
                    };
                    Ok(#name {
                        #(#field_builders)*
                    })
                }
            }
            Fields::Unnamed(fields) => {
                let field_builders = fields.unnamed.iter().enumerate().map(|(idx, f)| {
                    let field_ty = &f.ty;
                    let is_option = is_option_type(field_ty);

                    if is_option {
                        quote! {
                            {
                                match items.get(#idx) {
                                    None | Some(::tycl_parser::value::Value::Null) => None,
                                    Some(v) => {
                                        <#field_ty as ::tycl_parser::value::TryFromValue>::try_from_value(v.clone())?
                                    }
                                }
                            },
                        }
                    } else {
                        quote! {
                            {
                                let v = items.get(#idx).cloned().ok_or_else(|| {
                                    ::tycl_parser::error::ValueAccessError::TupleIndexOutOfBounds {
                                        index: #idx,
                                        len: items.len(),
                                    }
                                })?;
                                <#field_ty as ::tycl_parser::value::TryFromValue>::try_from_value(v)?
                            },
                        }
                    }
                });

                quote! {
                    let items = match value {
                        ::tycl_parser::value::Value::Tuple(items) => items,
                        _ => {
                            return Err(::tycl_parser::error::ValueAccessError::TypeMismatch {
                                expected: "tuple",
                                found: value.type_name(),
                            });
                        }
                    };
                    Ok(#name(
                        #(#field_builders)*
                    ))
                }
            }
            Fields::Unit => {
                quote! {
                    match value {
                        ::tycl_parser::value::Value::Record(_) | ::tycl_parser::value::Value::Tuple(_) => Ok(#name),
                        _ => Err(::tycl_parser::error::ValueAccessError::TypeMismatch {
                            expected: "record or tuple",
                            found: value.type_name(),
                        }),
                    }
                }
            }
        },
        Data::Enum(data) => {
            let variant_matchers = data.variants.iter().map(|v| {
                let variant_ident = &v.ident;
                let rename = get_tycl_rename(&v.attrs);
                let variant_str = rename.unwrap_or_else(|| variant_ident.to_string());
                quote! {
                    #variant_str => Ok(#name::#variant_ident),
                }
            });

            quote! {
                let s = match value {
                    ::tycl_parser::value::Value::String(s) => s,
                    _ => {
                        return Err(::tycl_parser::error::ValueAccessError::TypeMismatch {
                            expected: "string",
                            found: value.type_name(),
                        });
                    }
                };
                match s.as_str() {
                    #(#variant_matchers)*
                    _ => Err(::tycl_parser::error::ValueAccessError::TypeMismatch {
                        expected: "enum variant",
                        found: s,
                    }),
                }
            }
        }
        _ => {
            return syn::Error::new_spanned(
                input,
                "TryFromValue can only be derived for structs and enums",
            )
            .to_compile_error()
            .into();
        }
    };

    let expanded = quote! {
        impl #impl_generics ::tycl_parser::value::TryFromValue for #name #ty_generics #where_clause {
            fn try_from_value(value: ::tycl_parser::value::Value) -> ::std::result::Result<Self, ::tycl_parser::error::ValueAccessError> {
                #body
            }
        }
    };

    TokenStream::from(expanded)
}
