use crate::trace::lower::TracedItem;

use syn::spanned::Spanned;

/// Instruments a block of code.
///
/// This function generates the instrumented version of the given block. If the block is part of an async function,
/// it will be wrapped in an async block. Otherwise, the span will be entered and then the rest of the body will be performed.
///
/// # Examples
///
/// ```
/// // Assuming `block` is a syn::Block for the block `{ let x = 5; }`
/// // and `traced_item` is a TracedItem with name "my_func" and `enter_on_poll` set to false
/// let instrumented_block = gen_block(&block, false, traced_item);
/// ```
///
/// # Errors
///
/// This function will return an error if `enter_on_poll` is true but the function is not async.
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Panics
///
/// This function does not panic under normal conditions.
///
/// # Lifetimes
///
/// `'a` - The lifetime of the reference to the `syn::Block` in the `block` argument.
///
/// # Arguments
///
/// `block` - A reference to a `syn::Block` that should be instrumented.
///
/// `async_context` - A boolean indicating whether the block is part of an async function.
///
/// `traced_item` - A `TracedItem` containing the name of the span and whether it should be entered on poll.
///
/// # Notes
///
/// The function generates the instrumented function body. If the function is an `async fn`, this will wrap it in an async block.
/// Otherwise, this will enter the span and then perform the rest of the body.
pub fn gen_block(
    block: &syn::Block,
    async_context: bool,
    traced_item: TracedItem,
) -> proc_macro2::TokenStream {
    let event = traced_item.name.value();

    // Generate the instrumented function body.
    // If the function is an `async fn`, this will wrap it in an async block.
    // Otherwise, this will enter the span and then perform the rest of the body.
    if async_context {
        if traced_item.enter_on_poll.value {
            quote::quote_spanned!(block.span()=>
                minitrace::future::FutureExt::enter_on_poll(
                    async move { #block },
                    #event
                )
            )
        } else {
            quote::quote_spanned!(block.span()=>
                minitrace::future::FutureExt::in_span(
                    async move { #block },
                    minitrace::Span::enter_with_local_parent( #event )
                )
            )
        }
    } else {
        if traced_item.enter_on_poll.value {
            let e = syn::Error::new(
                syn::spanned::Spanned::span(&async_context),
                "`enter_on_poll` can not be applied on non-async function",
            );
            let tokens = quote::quote_spanned!(block.span()=>
                let __guard = minitrace::local::LocalSpan::enter_with_local_parent( #event );
                #block
            );
            return crate::token_stream_with_error(tokens, e);
        }

        quote::quote_spanned!(block.span()=>
            let __guard = minitrace::local::LocalSpan::enter_with_local_parent( #event );
            #block
        )
    }
}
