// Rust MCP
// Copyright (C) 2025 Jade Harley
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of  MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
// more details.
//
// You should have received a copy of the GNU General Public License along with
// this program.  If not, see <http://www.gnu.org/licenses/>.
use std::env;
use std::fs;
use std::path::Path;
use typify::{TypeSpace, TypeSpacePatch, TypeSpaceSettings};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.lock");
    let content = std::fs::read_to_string("specification/schema/2024-11-05/schema.json").unwrap();
    let schema = serde_json::from_str::<schemars::schema::RootSchema>(&content).unwrap();

    // Set up the type space
    let mut settings = TypeSpaceSettings::default();
    let settings = settings
        .with_struct_builder(false)
        .with_derive("::yoke::Yokeable".to_string())
        // Result... really?
        .with_patch(
            "Result",
            TypeSpacePatch::default().with_rename("ResultData"),
        );
    let blacklisted_types = ["Cursor"];
    for blacklisted_type in blacklisted_types {
        settings.with_replacement(blacklisted_type, blacklisted_type, vec![].into_iter());
    }

    let mut type_space = TypeSpace::new(settings);
    type_space.add_root_schema(schema).unwrap();

    let mut parsed = syn::parse2::<syn::File>(type_space.to_stream()).unwrap();
    zerocopify::transform_ast(&mut parsed, &["ProgressToken", "RequestId", "Result"]);
    let contents = prettyplease::unparse(&parsed);

    let mut out_file = Path::new(&env::var("OUT_DIR").unwrap()).to_path_buf();
    out_file.push("schema.rs");
    fs::write(out_file, contents).unwrap();
}

mod zerocopify {
    use std::collections::HashSet;
    use syn::{
        parse::Parser,
        parse_quote,
        punctuated::Punctuated,
        visit_mut::{self, VisitMut},
        Attribute, Fields, File, GenericParam, Lifetime, LifetimeParam, Meta, Token, Type,
        TypePath,
    };
    /// Returns true if the type (or any nested type) contains a lifetime.
    fn type_contains_lifetime(ty: &Type) -> bool {
        match ty {
            Type::Reference(r) => r.lifetime.is_some() || type_contains_lifetime(&r.elem),
            Type::Path(tp) => {
                for seg in &tp.path.segments {
                    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                        for arg in &ab.args {
                            match arg {
                                syn::GenericArgument::Lifetime(_) => return true,
                                syn::GenericArgument::Type(inner_ty) => {
                                    if type_contains_lifetime(inner_ty) {
                                        return true;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                false
            }
            Type::Group(g) => type_contains_lifetime(&g.elem),
            Type::Tuple(t) => t.elems.iter().any(type_contains_lifetime),
            Type::Array(a) => type_contains_lifetime(&a.elem),
            _ => false,
        }
    }

    /// Returns true if any attribute in `attrs` is a `derive` that includes `Deserialize`.
    fn derives_deserialize(attrs: &[Attribute]) -> bool {
        attrs.iter().any(|attr| {
            if attr.path().is_ident("derive") {
                if let Meta::List(meta_list) = &attr.meta {
                    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
                    if let Ok(metas) = parser.parse2(meta_list.tokens.clone()) {
                        return metas.into_iter().any(|m| match m {
                            Meta::Path(ref path) => path
                                .segments
                                .last()
                                .map_or(false, |seg| seg.ident == "Deserialize"),
                            Meta::List(ref list) => list
                                .path
                                .segments
                                .last()
                                .map_or(false, |seg| seg.ident == "Deserialize"),
                            Meta::NameValue(ref nv) => nv
                                .path
                                .segments
                                .last()
                                .map_or(false, |seg| seg.ident == "Deserialize"),
                        });
                    }
                }
            }
            false
        })
    }
    /// (Optional) Returns true if `ty` uses any type in `changed_types`.
    fn type_uses_changed_type(ty: &Type, changed_types: &HashSet<String>) -> bool {
        match ty {
            Type::Path(tp) => {
                for seg in &tp.path.segments {
                    if changed_types.contains(&seg.ident.to_string()) {
                        return true;
                    }
                    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                        for arg in &ab.args {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                if type_uses_changed_type(inner_ty, changed_types) {
                                    return true;
                                }
                            }
                        }
                    }
                }
                false
            }
            Type::Reference(r) => type_uses_changed_type(&r.elem, changed_types),
            _ => false,
        }
    }

    /// Returns true if the type is a reference (like `&T` or `&str`).
    fn is_reference_type(ty: &Type) -> bool {
        matches!(ty, Type::Reference(_))
    }

    /// Returns true if the provided `TypePath` is exactly `String`.
    fn is_string_type(type_path: &TypePath) -> bool {
        type_path
            .path
            .segments
            .last()
            .map(|seg| seg.ident == "String")
            .unwrap_or(false)
    }

    /// Transforms a field by:
    /// 1. Replacing a bare `String` with `&'a str`.
    /// 2. If the field’s type (or any inner type) contains a lifetime (even inside a generic) and isn’t a reference,
    ///    ensuring that a `#[serde(borrow)]` attribute is attached.
    /// Returns `true` if the field “requires” a lifetime.
    fn transform_field(field: &mut syn::Field) -> bool {
        let mut modified = false;

        match &mut field.ty {
            Type::Path(type_path) if is_string_type(type_path) => {
                field.ty = parse_quote!(&'a str);
                modified = true;
            }
            Type::Path(type_path) => {
                // Skip if it's a JSON Map
                if let Some(last_seg) = type_path.path.segments.last() {
                    if last_seg.ident == "Map" {
                        return false;
                    }
                }
                if let Some(last_seg) = type_path.path.segments.last_mut() {
                    if let syn::PathArguments::AngleBracketed(args) = &mut last_seg.arguments {
                        for arg in &mut args.args {
                            if let syn::GenericArgument::Type(inner_ty) = arg {
                                if let Type::Path(inner_path) = inner_ty {
                                    if is_string_type(inner_path) {
                                        {
                                            *inner_ty = parse_quote!(&'a str);
                                            modified = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        if !is_reference_type(&field.ty) && type_contains_lifetime(&field.ty) {
            let already_has_borrow = field
                .attrs
                .iter()
                .any(|attr| quote::quote!(#attr).to_string().contains("borrow"));
            if !already_has_borrow {
                field.attrs.push(parse_quote!(#[serde(borrow)]));
                modified = true;
            }
        }
        modified
    }

    /// Our primary transformer visitor.
    struct SerdeBorrowTransformer {
        /// Names of structs to not transform
        ignored_types: Vec<&'static str>,
        /// Names of types that have been updated to include a lifetime.
        changed_types: HashSet<String>,
        /// Whether any change was made in this pass.
        modified: bool,
    }

    impl SerdeBorrowTransformer {
        fn new(ignored_types: &[&'static str]) -> Self {
            Self {
                ignored_types: ignored_types.to_vec(),
                changed_types: HashSet::new(),
                modified: false,
            }
        }
    }

    impl VisitMut for SerdeBorrowTransformer {
        fn visit_item_struct_mut(&mut self, item: &mut syn::ItemStruct) {
            if !self
                .ignored_types
                .contains(&item.ident.to_string().as_str())
                && derives_deserialize(&item.attrs)
            {
                let mut changed_any = false;
                match &mut item.fields {
                    Fields::Named(fields_named) => {
                        for field in fields_named.named.iter_mut() {
                            if transform_field(field) {
                                changed_any = true;
                            }
                            if type_uses_changed_type(&field.ty, &self.changed_types) {
                                changed_any = true;
                            }
                        }
                    }
                    Fields::Unnamed(fields_unnamed) => {
                        for field in fields_unnamed.unnamed.iter_mut() {
                            if transform_field(field) {
                                changed_any = true;
                            }
                            if type_uses_changed_type(&field.ty, &self.changed_types) {
                                changed_any = true;
                            }
                        }
                    }
                    Fields::Unit => {}
                }
                if changed_any {
                    let lifetime_a: Lifetime = parse_quote! {'a};
                    if !item
                        .generics
                        .lifetimes()
                        .any(|lt| lt.lifetime == lifetime_a)
                    {
                        let lifetime_param = LifetimeParam {
                            attrs: Vec::new(),
                            lifetime: lifetime_a.clone(),
                            colon_token: None,
                            bounds: Punctuated::new(),
                        };
                        item.generics
                            .params
                            .insert(0, GenericParam::Lifetime(lifetime_param));
                        self.modified = true;
                    }
                    self.changed_types.insert(item.ident.to_string());
                }
            }
            visit_mut::visit_item_struct_mut(self, item);
        }

        fn visit_item_enum_mut(&mut self, item: &mut syn::ItemEnum) {
            // Only process enums that derive Deserialize.
            if !derives_deserialize(&item.attrs)
                || self
                    .ignored_types
                    .contains(&item.ident.to_string().as_str())
            {
                return visit_mut::visit_item_enum_mut(self, item);
            }

            let mut requires_lifetime = false;
            // Process every variant
            for variant in item.variants.iter_mut() {
                match &mut variant.fields {
                    Fields::Named(named_fields) => {
                        for field in named_fields.named.iter_mut() {
                            if transform_field(field)
                                || (!is_reference_type(&field.ty)
                                    && type_contains_lifetime(&field.ty))
                            {
                                requires_lifetime = true;
                            }
                        }
                    }
                    Fields::Unnamed(unnamed_fields) => {
                        for field in unnamed_fields.unnamed.iter_mut() {
                            if transform_field(field)
                                || (!is_reference_type(&field.ty)
                                    && type_contains_lifetime(&field.ty))
                            {
                                requires_lifetime = true;
                            }
                        }
                    }
                    Fields::Unit => { /* nothing to check */ }
                }
            }
            if requires_lifetime {
                // If the enum doesn't already have any lifetime parameter, add one.
                if item.generics.lifetimes().next().is_none() {
                    let lifetime_a: Lifetime = parse_quote! {'a};
                    let lifetime_param = LifetimeParam {
                        attrs: Vec::new(),
                        lifetime: lifetime_a.clone(),
                        colon_token: None,
                        bounds: Punctuated::new(),
                    };
                    item.generics
                        .params
                        .insert(0, GenericParam::Lifetime(lifetime_param));
                    self.modified = true;
                }
                self.changed_types.insert(item.ident.to_string());
            }
            visit_mut::visit_item_enum_mut(self, item);
        }

        fn visit_item_impl_mut(&mut self, item_impl: &mut syn::ItemImpl) {
            if impl_contains_changed_type(item_impl, &self.changed_types) {
                let lifetime_a: Lifetime = parse_quote! {'a};
                if !item_impl
                    .generics
                    .lifetimes()
                    .any(|lt| lt.lifetime == lifetime_a)
                {
                    let lifetime_param = LifetimeParam {
                        attrs: Vec::new(),
                        lifetime: lifetime_a.clone(),
                        colon_token: None,
                        bounds: Punctuated::new(),
                    };
                    item_impl
                        .generics
                        .params
                        .insert(0, GenericParam::Lifetime(lifetime_param));
                    self.modified = true;
                }
            }
            visit_mut::visit_item_impl_mut(self, item_impl);
        }

        fn visit_type_path_mut(&mut self, type_path: &mut TypePath) {
            if let Some(last_seg) = type_path.path.segments.last_mut() {
                if self.changed_types.contains(&last_seg.ident.to_string()) {
                    match &mut last_seg.arguments {
                        syn::PathArguments::None => {
                            let angle_bracketed: syn::AngleBracketedGenericArguments =
                                parse_quote!(<'a>);
                            last_seg.arguments =
                                syn::PathArguments::AngleBracketed(angle_bracketed);
                            self.modified = true;
                        }
                        syn::PathArguments::AngleBracketed(gen_args) => {
                            if gen_args.args.is_empty() {
                                gen_args.args.push(parse_quote!('a));
                                self.modified = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
            visit_mut::visit_type_path_mut(self, type_path);
        }
    }
    /// Checks if an impl’s self type (or trait) uses a changed type.
    fn impl_contains_changed_type(
        item_impl: &syn::ItemImpl,
        changed_types: &HashSet<String>,
    ) -> bool {
        // Check the self type recursively.
        if type_uses_changed_type(&*item_impl.self_ty, changed_types) {
            return true;
        }
        // Check the trait, if present.
        if let Some((_, trait_path, _)) = &item_impl.trait_ {
            let ty: Type = Type::Path(syn::TypePath {
                qself: None,
                path: trait_path.clone(),
            });
            if type_uses_changed_type(&ty, changed_types) {
                return true;
            }
        }
        false
    }

    /// After phase 1, collect the names of all structs and enums that now have lifetime parameters.
    fn collect_lifetime_types(ast: &File) -> HashSet<String> {
        let mut set = HashSet::new();
        for item in &ast.items {
            match item {
                syn::Item::Struct(item_struct) => {
                    if item_struct.generics.lifetimes().next().is_some() {
                        set.insert(item_struct.ident.to_string());
                    }
                }
                syn::Item::Enum(item_enum) => {
                    if item_enum.generics.lifetimes().next().is_some() {
                        set.insert(item_enum.ident.to_string());
                    }
                }
                _ => {}
            }
        }
        set
    }

    /// A second-pass transformer that ensures every usage of a type that has a lifetime parameter
    /// actually provides a lifetime argument.
    struct LifetimeUsageTransformer {
        lifetime_types: HashSet<String>,
        modified: bool,
    }

    impl VisitMut for LifetimeUsageTransformer {
        fn visit_type_path_mut(&mut self, type_path: &mut TypePath) {
            if let Some(last_seg) = type_path.path.segments.last_mut() {
                if self.lifetime_types.contains(&last_seg.ident.to_string()) {
                    match &mut last_seg.arguments {
                        syn::PathArguments::None => {
                            let angle_bracketed: syn::AngleBracketedGenericArguments =
                                syn::parse_quote!(<'a>);
                            last_seg.arguments =
                                syn::PathArguments::AngleBracketed(angle_bracketed);
                            self.modified = true;
                        }
                        syn::PathArguments::AngleBracketed(gen_args) => {
                            if gen_args.args.is_empty() {
                                gen_args.args.push(syn::parse_quote!('a));
                                self.modified = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
            visit_mut::visit_type_path_mut(self, type_path);
        }
    }

    /// Applies both transformation phases to the AST.
    pub fn transform_ast(ast: &mut File, ignored_types: &[&'static str]) {
        loop {
            let mut modified = false;

            // Phase 1: Upgrade definitions and usages.
            {
                let mut transformer = SerdeBorrowTransformer::new(ignored_types);
                transformer.visit_file_mut(ast);
                modified |= transformer.modified;
            }

            // Phase 2: Ensure every usage of a type that now has a lifetime parameter provides one.
            {
                let lifetime_types = collect_lifetime_types(ast);
                let mut transformer = LifetimeUsageTransformer {
                    lifetime_types,
                    modified: false,
                };
                transformer.visit_file_mut(ast);
                modified |= transformer.modified;
            }

            if !modified {
                break;
            }
        }
    }
}
