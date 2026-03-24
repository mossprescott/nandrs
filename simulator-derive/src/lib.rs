use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type, parse_macro_input};

/// Derive `simulator::Reflect` for a struct or enum.
///
/// ## On structs
///
/// Fields whose type name starts with `Input` are treated as input ports.
/// Fields whose type name starts with `Output` are treated as output ports.
/// All other fields are ignored.
///
/// ```ignore
/// #[derive(Reflect)]
/// pub struct And {
///     pub a: Input,
///     pub b: Input,
///     pub out: Output,
/// }
/// ```
///
/// ## On enums
///
/// Each variant must be a single-field tuple variant. Delegates `reflect()` and `name()`
/// to the inner value.
///
/// ```ignore
/// #[derive(Reflect)]
/// pub enum MyComponent {
///     And(And),
///     Or(Or),
/// }
/// ```
#[proc_macro_derive(Reflect)]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    match &input.data {
        Data::Struct(data_struct) => derive_reflect_struct(name, &input, data_struct),
        Data::Enum(data_enum) => derive_reflect_enum(name, &input, data_enum),
        _ => panic!("Reflect can only be derived for structs and enums"),
    }
}

fn derive_reflect_struct(
    name: &syn::Ident,
    input: &DeriveInput,
    data_struct: &syn::DataStruct,
) -> TokenStream {
    let Fields::Named(named_fields) = &data_struct.fields else {
        panic!("Reflect can only be derived for structs with named fields");
    };

    let mut inputs = vec![];
    let mut outputs = vec![];

    for field in &named_fields.named {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let field_ty = &field.ty;

        if type_name_starts_with(field_ty, "Input") {
            inputs.push(quote! {
                (#field_name_str.to_string(), BusRef::from_input(self.#field_name))
            });
        } else if type_name_starts_with(field_ty, "Output") {
            outputs.push(quote! {
                (#field_name_str.to_string(), BusRef::from_output(self.#field_name))
            });
        }
    }

    let name_str = name.to_string();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics Reflect for #name #ty_generics #where_clause {
            fn reflect(&self) -> Interface {
                Interface {
                    inputs:  ::std::collections::HashMap::from([#(#inputs),*]),
                    outputs: ::std::collections::HashMap::from([#(#outputs),*]),
                }
            }
            fn name(&self) -> String { #name_str.into() }
        }
    }
    .into()
}

fn derive_reflect_enum(
    name: &syn::Ident,
    input: &DeriveInput,
    data_enum: &syn::DataEnum,
) -> TokenStream {
    let variants = parse_enum_variants(data_enum, name);

    let reflect_arms: Vec<_> = variants
        .iter()
        .map(|(vname, _)| {
            quote! { Self::#vname(c) => c.reflect() }
        })
        .collect();

    let name_arms: Vec<_> = variants
        .iter()
        .map(|(vname, _)| {
            quote! { Self::#vname(c) => c.name() }
        })
        .collect();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics Reflect for #name #ty_generics #where_clause {
            fn reflect(&self) -> Interface {
                match self {
                    #(#reflect_arms,)*
                }
            }
            fn name(&self) -> String {
                match self {
                    #(#name_arms,)*
                }
            }
        }
    }
    .into()
}

/// Derive `simulator::Chip` for a struct whose fields are all `Input*`/`Output*` buses.
///
/// Generates a `chip()` constructor that calls `::new()` on every field.
///
/// ```ignore
/// #[derive(Chip)]
/// pub struct And {
///     pub a: Input,
///     pub b: Input,
///     pub out: Output,
/// }
/// ```
#[proc_macro_derive(Chip)]
pub fn derive_chip(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let Data::Struct(data_struct) = &input.data else {
        panic!("Chip can only be derived for structs");
    };
    let Fields::Named(named_fields) = &data_struct.fields else {
        panic!("Chip can only be derived for structs with named fields");
    };

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let chip_fields: Vec<_> = named_fields
        .named
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_ty = &field.ty;
            quote! { #field_name: <#field_ty>::new() }
        })
        .collect();

    quote! {
        impl #impl_generics Chip for #name #ty_generics #where_clause {
            fn chip() -> Self {
                Self { #(#chip_fields),* }
            }
        }
    }
    .into()
}

/// Derive `simulator::Component` for an enum, plus `From` impls for each variant.
///
/// Each variant must be a single-field tuple variant. Supported attributes:
///
/// - `#[primitive]` — the variant has no `Component` impl; `expand()` returns `None`.
/// - `#[delegate]` — the variant wraps another component enum; generates a blanket
///   `impl<C: Into<InnerType>> From<C> for Enum` instead of a simple `From<InnerType>`.
/// - (no attribute) — delegates `expand()` to the inner type, mapping the result
///   via `Into` to convert components to `Self`.
///
/// ```ignore
/// #[derive(Component)]
/// pub enum MyComponent {
///     #[primitive] Nand(Nand),
///     #[primitive] Buffer(Buffer),
///     #[delegate] Project01(Project01Component),
///     Not(Not),
///     And(And),
/// }
/// ```
#[proc_macro_derive(Component, attributes(primitive, delegate))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let Data::Enum(data_enum) = &input.data else {
        panic!("Component derive can only be used on enums");
    };

    let variants = parse_enum_variants(data_enum, name);

    // Classify variants by attribute
    let mut from_impls = vec![];
    let mut expand_arms = vec![];

    for (vname, inner_ty) in &variants {
        let is_primitive = has_attr(&data_enum.variants, vname, "primitive");
        let is_delegate = has_attr(&data_enum.variants, vname, "delegate");

        // From impls
        if is_delegate {
            from_impls.push(quote! {
                impl<C: Into<#inner_ty>> From<C> for #name {
                    fn from(c: C) -> Self {
                        #name::#vname(c.into())
                    }
                }
            });
        } else {
            from_impls.push(quote! {
                impl From<#inner_ty> for #name {
                    fn from(c: #inner_ty) -> Self {
                        #name::#vname(c)
                    }
                }
            });
        }

        // Component::expand arms
        if is_primitive {
            expand_arms.push(quote! {
                #name::#vname(_) => None
            });
        } else {
            expand_arms.push(quote! {
                #name::#vname(c) => c.expand().map(|ic| IC {
                    name: ic.name,
                    intf: ic.intf,
                    components: ic.components.into_iter().map(Into::into).collect(),
                })
            });
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        #(#from_impls)*

        impl #impl_generics Component for #name #ty_generics #where_clause {
            type Target = #name #ty_generics;

            fn expand(&self) -> Option<IC<Self::Target>> {
                match self {
                    #(#expand_arms,)*
                }
            }
        }
    }
    .into()
}

/// Parse enum variants, ensuring each is a single-field tuple variant.
/// Returns (variant_ident, inner_type) pairs.
fn parse_enum_variants(
    data_enum: &syn::DataEnum,
    enum_name: &syn::Ident,
) -> Vec<(syn::Ident, Type)> {
    data_enum
        .variants
        .iter()
        .map(|v| {
            let Fields::Unnamed(fields) = &v.fields else {
                panic!("{}::{} must be a tuple variant", enum_name, v.ident);
            };
            if fields.unnamed.len() != 1 {
                panic!("{}::{} must have exactly one field", enum_name, v.ident);
            }
            (v.ident.clone(), fields.unnamed[0].ty.clone())
        })
        .collect()
}

/// Check whether a variant has a given attribute (e.g. `#[primitive]`).
fn has_attr(
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
    vname: &syn::Ident,
    attr_name: &str,
) -> bool {
    variants
        .iter()
        .find(|v| v.ident == *vname)
        .map(|v| v.attrs.iter().any(|a| a.path().is_ident(attr_name)))
        .unwrap_or(false)
}

fn type_name_starts_with(ty: &Type, prefix: &str) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident.to_string().starts_with(prefix);
        }
    }
    false
}
