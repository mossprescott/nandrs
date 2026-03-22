use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type, parse_macro_input};

/// Derive `simulator::Reflect` for a struct whose fields are named `Input*`/`Output*` buses.
///
/// Fields whose type name starts with `Input` are treated as input ports.
/// Fields whose type name starts with `Output` are treated as output ports.
/// All other fields are ignored.
///
/// # Example
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
/// expands to:
///
/// ```ignore
/// impl simulator::Reflect for And {
///     fn reflect(&self) -> simulator::Interface {
///         simulator::Interface {
///             inputs: HashMap::from([
///                 ("a".to_string(), self.a.clone().into()),
///                 ("b".to_string(), self.b.clone().into()),
///             ]),
///             outputs: HashMap::from([
///                 ("out".to_string(), self.out.clone().into()),
///             ]),
///         }
///     }
///     fn name(&self) -> &str { "And" }
/// }
/// ```
#[proc_macro_derive(Reflect)]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let Data::Struct(data_struct) = &input.data else {
        panic!("Reflect can only be derived for structs");
    };
    let Fields::Named(named_fields) = &data_struct.fields else {
        panic!("Reflect can only be derived for structs with named fields");
    };

    let mut inputs  = vec![];
    let mut outputs = vec![];

    for field in &named_fields.named {
        let field_name     = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let field_ty       = &field.ty;

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

/// Derive `simulator::Chip` for a struct whose fields are all `Input*`/`Output*` buses.
///
/// Generates a `chip()` constructor that calls `::new()` on every field.
///
/// # Example
///
/// ```ignore
/// #[derive(Chip)]
/// pub struct And {
///     pub a: Input,
///     pub b: Input,
///     pub out: Output,
/// }
/// ```
///
/// expands to:
///
/// ```ignore
/// impl simulator::Chip for And {
///     fn chip() -> Self {
///         Self { a: Input::new(), b: Input::new(), out: Output::new() }
///     }
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

    let chip_fields: Vec<_> = named_fields.named.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_ty   = &field.ty;
        quote! { #field_name: <#field_ty>::new() }
    }).collect();

    quote! {
        impl #impl_generics Chip for #name #ty_generics #where_clause {
            fn chip() -> Self {
                Self { #(#chip_fields),* }
            }
        }
    }
    .into()
}

fn type_name_starts_with(ty: &Type, prefix: &str) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident.to_string().starts_with(prefix);
        }
    }
    false
}
