#[allow(unused_imports)]
mod async_trait;
mod block;
mod lifetime;
pub mod quotable;
mod signature;

use quote::quote;

use crate::trace::analyze::Model;
use crate::trace::analyze::Models;
use crate::trace::analyze::TracedItem;

use crate::trace::lower::async_trait::*;
use crate::trace::lower::block::*;
use crate::trace::lower::quotable::*;
use crate::trace::lower::signature::*;

use syn::visit_mut::VisitMut;

/// Lowers the `Models<Model>` into a `Quotables<Quotable>` collection.
///
/// The `lower` function is responsible for transforming the high-level `Models<Model>` representation into a lower-level `Quotables<Quotable>` representation, or intermediate representation, that can be processed by the `quote::quote::quote_spanned!()` macro.
/// Quotables is a Vec-newtype, implemented in the same way as `Models<Model>`.
///
/// # Examples
///
/// ```
/// // Assuming `models` is a Models<Model> with at least one Model::Item
/// let quotes = lower(models);
/// assert!(quotes.len() > 0);
/// ```
///
/// # Panics
///
/// This function will panic if `models` does not contain at least one `Model::Item`.
///
/// # Arguments
///
/// `models` - A `Models<Model>` object. This should contain at least one `Model::Item`.
pub fn lower(models: Models<Model>) -> Quotables<Quotable> {
    let mut quotes = Quotables::new();
    quotes.extend(models.iter().map(|model| {
        let traced_item = if let Model::Item(ti) = model {
            Ok(ti)
        } else {
            Err(())
        }
        .unwrap();
        Quotable::Item(quote(*(*traced_item).clone()))
    }));
    quotes
}

/// Transforms a `TracedItem` into a `Quote`.
///
/// The `quote` function is responsible for transforming the high-level `TracedItem` representation into a lower-level `Quote` representation that can be processed by the `quote::quote::quote_spanned!()` macro.
///
/// # Examples
///
/// ```
/// // Assuming `traced_item` is a TracedItem with a valid ItemFn
/// let quote = quote(traced_item);
/// assert_eq!(quote.ident, "my_function");
/// ```
///
/// # Panics
///
/// This function will panic if `traced_item` does not contain a valid `ItemFn`.
///
/// # Arguments
///
/// `traced_item` - A `TracedItem` object. This should contain a valid `ItemFn`.
pub fn quote(traced_item: TracedItem) -> Quote {
    let input = traced_item.item_fn.clone();

    // check for async_trait-like patterns in the block, and instrument
    // the future instead of the wrapper
    let func_body = if let Some(internal_fun) =
        get_async_trait_info(&input.block, input.sig.asyncness.is_some())
    {
        // let's rewrite some statements!
        match internal_fun.kind {
            // async-trait <= 0.1.43
            AsyncTraitKind::Function(_) => {
                unimplemented!(
                    "Please upgrade the crate `async-trait` to a version higher than 0.1.44"
                )
            }
            // async-trait >= 0.1.44
            AsyncTraitKind::Async(async_expr) => {
                // fallback if we couldn't find the '__async_trait' binding, might be
                // useful for crates exhibiting the same behaviors as async-trait
                let instrumented_block = gen_block(&async_expr.block, true, traced_item);
                let async_attrs = &async_expr.attrs;
                quote! {
                        Box::pin(#(#async_attrs) * { #instrumented_block })
                }
            }
        }
    } else {
        gen_block(&input.block, input.sig.asyncness.is_some(), traced_item)
    };

    let syn::ItemFn {
        attrs,
        vis,
        mut sig,
        ..
    } = input;

    if sig.asyncness.is_some() {
        let has_self = has_self_in_sig(&mut sig);
        transform_sig(&mut sig, has_self, true);
    }

    let syn::Signature {
        output: return_type,
        inputs: params,
        unsafety,
        constness,
        abi,
        ident,
        generics:
            syn::Generics {
                params: gen_params,
                where_clause,
                ..
            },
        ..
    } = sig;

    Quote {
        attrs,
        vis,
        constness,
        unsafety,
        abi,
        ident,
        gen_params,
        params,
        return_type,
        where_clause,
        func_body,
    }
}

/// Checks if a function signature contains `self`.
///
/// This function uses a visitor pattern to traverse the syntax tree of the function signature and checks if it contains `self`.
///
/// # Examples
///
/// ```
/// // Assuming `sig` is a syn::Signature for the function `fn foo(&self)`
/// assert_eq!(has_self_in_sig(&mut sig), true);
/// ```
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Panics
///
/// This function does not panic under normal conditions.
///
/// # Arguments
///
/// `sig` - A mutable reference to a `syn::Signature` object. This represents the signature of a function.
fn has_self_in_sig(sig: &mut syn::Signature) -> bool {
    let mut visitor = HasSelf(false);
    visitor.visit_signature_mut(sig);
    visitor.0
}

#[cfg(test)]
mod tests {
    use test_utilities::*;

    #[test]
    fn sync_quote_1() {
        let ts: syn::ItemFn = syn::parse_quote!(
            fn f() {}
        );
        //let args: Vec<syn::NestedMeta> = vec![];
        let trace = crate::trace::Trace {
            ..Default::default()
        };

        let models = crate::trace::analyze(trace, quote::ToTokens::into_token_stream(ts));

        let quotes = crate::trace::lower(models);

        let expected = crate::trace::lower::Quotable::Item(crate::trace::lower::Quote {
            attrs: Vec::new(),
            vis: syn::Visibility::Inherited,
            constness: None,
            unsafety: None,
            abi: None,
            ident: syn::Ident::new("f", proc_macro2::Span::call_site()),
            gen_params: syn::punctuated::Punctuated::new(),
            params: syn::punctuated::Punctuated::new(),
            return_type: syn::ReturnType::Default,
            where_clause: None,
            func_body: quote::quote!(
                let __guard = minitrace::local::LocalSpan::enter_with_local_parent("f");
                {}
            ),
        });

        let actual = format!("{:#?}", quotes.get(0).unwrap());
        assert_eq_text!(&format!("{:#?}", expected), &actual);
    }
}
