use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Meta};

/// A derive macro that generates a method to convert a struct into a JSON Schema-like
/// representation. The outer structure is a HashMap, while the inner structure uses
/// serde_json::Map for compatibility with JSON values.
#[proc_macro_derive(ToolQuery)]
pub fn schema_gen(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Extract fields from struct
    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("SchemaGen only supports structs with named fields"),
        },
        _ => panic!("SchemaGen only supports structs"),
    };

    // Generate field mappings
    let field_mappings = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap().to_string();
        let field_type = get_type_string(&field.ty);
        
        // Extract doc comments, preserving all lines
        let docs = field.attrs.iter()
            .filter(|attr| attr.path().is_ident("doc"))
            .filter_map(|attr| {
                if let Meta::NameValue(ref meta) = attr.meta {
                    if let syn::Expr::Lit(expr_lit) = &meta.value {
                        if let syn::Lit::Str(lit) = &expr_lit.lit {
                            Some(lit.value())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
            .trim()
            .to_string();

        quote! {
            {
                let mut field_map = serde_json::Map::new();
                field_map.insert("type".to_string(), serde_json::Value::String(#field_type.to_string()));
                field_map.insert("description".to_string(), serde_json::Value::String(#docs.to_string()));
                map.insert(#field_name.to_string(), field_map);
            }
        }
    });

    // Generate the implementation
    let expanded = quote! {
        impl #name {
            /// Generates a query tool schema representation of the struct's fields
            pub fn generate_schema() -> std::collections::HashMap<String, serde_json::Map<String, serde_json::Value>> {
                let mut map = std::collections::HashMap::new();
                #(#field_mappings)*
                map
            }
        }
    };

    TokenStream::from(expanded)
}

/// Helper function to convert Rust types to JSON Schema types
fn get_type_string(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => {
            let segment = type_path.path.segments.last().unwrap();
            match segment.ident.to_string().as_str() {
                "String" | "str" => "string".to_string(),
                "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64" => {
                    "number".to_string()
                }
                "bool" => "boolean".to_string(),
                "Vec" => "array".to_string(),
                "Option" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(arg) = args.args.first() {
                            if let syn::GenericArgument::Type(ty) = arg {
                                return get_type_string(ty);
                            }
                        }
                    }
                    "unknown".to_string()
                }
                _ => "object".to_string(),
            }
        }
        _ => "unknown".to_string(),
    }
}
