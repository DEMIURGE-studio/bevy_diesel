use proc_macro::TokenStream;
use proc_macro_crate::{crate_name, FoundCrate};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

fn found_path(found: FoundCrate) -> proc_macro2::TokenStream {
    match found {
        FoundCrate::Itself => quote! { ::bevy_diesel },
        FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote! { ::#ident }
        }
    }
}

/// Path prefix for the `bevy_diesel` crate. A consumer always depends on
/// `bevy_diesel` directly (it is the ecosystem umbrella), so this resolves to
/// `::bevy_diesel` downstream and `crate` inside diesel itself.
fn diesel_root() -> proc_macro2::TokenStream {
    match crate_name("bevy_diesel") {
        Ok(found) => found_path(found),
        Err(_) => quote! { ::bevy_diesel },
    }
}

/// Derive macro that implements [`PropagatedMessage`] for a message struct and
/// auto-registers its buffer, subscription graph, and propagation system via
/// `inventory`.
///
/// Mark the `Entity` field the message is addressed to with `#[propagate(target)]`.
/// The struct must also derive `Message`, `Clone`, and `Reflect` (to meet
/// `PropagatedMessage`'s supertrait bounds and the registration bounds); this
/// derive intentionally does not inject them for you.
///
/// # Example
///
/// ```ignore
/// use bevy::prelude::*;
/// use bevy_diesel::prelude::*;
///
/// #[derive(Message, Clone, Reflect, PropagatedMessage)]
/// struct Hit {
///     #[propagate(target)]
///     defender: Entity,
///     amount: f32,
/// }
/// ```
#[proc_macro_derive(PropagatedMessage, attributes(propagate))]
pub fn derive_propagated_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident.clone();

    let data = match &input.data {
        Data::Struct(s) => s,
        _ => {
            return syn::Error::new_spanned(
                &name,
                "#[derive(PropagatedMessage)] supports only structs",
            )
            .to_compile_error()
            .into();
        }
    };
    let fields = match &data.fields {
        Fields::Named(f) => &f.named,
        _ => {
            return syn::Error::new_spanned(
                &name,
                "#[derive(PropagatedMessage)] requires named fields; mark the addressed \
                 entity field with #[propagate(target)]",
            )
            .to_compile_error()
            .into();
        }
    };

    let mut target_field = None;
    for field in fields {
        for attr in &field.attrs {
            if !attr.path().is_ident("propagate") {
                continue;
            }
            let mut is_target = false;
            let res = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("target") {
                    is_target = true;
                    Ok(())
                } else {
                    Err(meta.error("unknown `propagate` field attribute; expected `target`"))
                }
            });
            if let Err(e) = res {
                return e.to_compile_error().into();
            }
            if is_target {
                if target_field.is_some() {
                    return syn::Error::new_spanned(
                        field,
                        "multiple #[propagate(target)] fields; exactly one is required",
                    )
                    .to_compile_error()
                    .into();
                }
                target_field = field.ident.clone();
            }
        }
    }

    let Some(target_field) = target_field else {
        return syn::Error::new_spanned(
            &name,
            "#[derive(PropagatedMessage)] requires exactly one field marked \
             #[propagate(target)] (the Entity the message is addressed to)",
        )
        .to_compile_error()
        .into();
    };

    let diesel = diesel_root();
    let expanded = quote! {
        const _: () = {
            use #diesel as _diesel;

            impl _diesel::propagation::PropagatedMessage for #name {
                fn target(&self) -> bevy::prelude::Entity {
                    self.#target_field
                }
                fn set_target(&mut self, entity: bevy::prelude::Entity) {
                    self.#target_field = entity;
                }
            }

            _diesel::inventory::submit! {
                _diesel::propagation::PropagationRegistrar {
                    register: |app| {
                        _diesel::propagation::register_propagation_for::<#name>(app);
                    },
                }
            }
        };
    };

    TokenStream::from(expanded)
}
