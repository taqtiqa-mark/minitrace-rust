use crate::trace::lower::lifetime::*;

use syn::visit_mut::VisitMut;

/// Transforms a function signature for async tracing.
///
/// This function modifies the provided function signature to enable async tracing. It adjusts the lifetimes, adds necessary bounds, and changes the return type to a Future.
///
/// # Arguments
///
/// * `sig` - The function signature to transform.
/// * `has_self` - A boolean indicating whether the function has a self parameter.
/// * `is_local` - A boolean indicating whether the function is local.
///
/// # Examples
///
/// ```
/// // Assuming `sig` is a mutable reference to a `syn::Signature` instance, `has_self` and `is_local` are booleans
/// transform_sig(&mut sig, has_self, is_local);
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
/// # Lifetimes
///
/// This function collects all lifetimes from the function signature and adjusts them for async tracing.
///
pub fn transform_sig(sig: &mut syn::Signature, has_self: bool, is_local: bool) {
    sig.fn_token.span = sig.asyncness.take().unwrap().span;

    let ret = match &sig.output {
        syn::ReturnType::Default => quote::quote!(()),
        syn::ReturnType::Type(_, ret) => quote::quote!(#ret),
    };

    let default_span = sig
        .ident
        .span()
        .join(sig.paren_token.span)
        .unwrap_or_else(|| sig.ident.span());

    let mut lifetimes = CollectLifetimes::new("'life", default_span);
    for arg in sig.inputs.iter_mut() {
        match arg {
            syn::FnArg::Receiver(arg) => lifetimes.visit_receiver_mut(arg),
            syn::FnArg::Typed(arg) => lifetimes.visit_type_mut(&mut arg.ty),
        }
    }

    for param in sig.generics.params.iter() {
        match param {
            syn::GenericParam::Type(param) => {
                let param = &param.ident;
                let span = param.span();
                where_clause_or_default(&mut sig.generics.where_clause)
                    .predicates
                    .push(syn::parse_quote_spanned!(span=> #param: 'minitrace));
            }
            syn::GenericParam::Lifetime(param) => {
                let param = &param.lifetime;
                let span = param.span();
                where_clause_or_default(&mut sig.generics.where_clause)
                    .predicates
                    .push(syn::parse_quote_spanned!(span=> #param: 'minitrace));
            }
            syn::GenericParam::Const(_) => {}
        }
    }

    if sig.generics.lt_token.is_none() {
        sig.generics.lt_token = Some(syn::Token![<](sig.ident.span()));
    }
    if sig.generics.gt_token.is_none() {
        sig.generics.gt_token = Some(syn::Token![>](sig.paren_token.span));
    }

    for (idx, elided) in lifetimes.elided.iter().enumerate() {
        sig.generics.params.insert(idx, syn::parse_quote!(#elided));
        where_clause_or_default(&mut sig.generics.where_clause)
            .predicates
            .push(syn::parse_quote_spanned!(elided.span()=> #elided: 'minitrace));
    }

    sig.generics
        .params
        .insert(0, syn::parse_quote_spanned!(default_span=> 'minitrace));

    if has_self {
        let bound_span = sig.ident.span();
        let bound = match sig.inputs.iter().next() {
            Some(syn::FnArg::Receiver(syn::Receiver {
                reference: Some(_),
                mutability: None,
                ..
            })) => syn::Ident::new("Sync", bound_span),
            Some(syn::FnArg::Typed(arg))
                if match (arg.pat.as_ref(), arg.ty.as_ref()) {
                    (syn::Pat::Ident(pat), syn::Type::Reference(ty)) => {
                        pat.ident == "self" && ty.mutability.is_none()
                    }
                    _ => false,
                } =>
            {
                syn::Ident::new("Sync", bound_span)
            }
            _ => syn::Ident::new("Send", bound_span),
        };

        let where_clause = where_clause_or_default(&mut sig.generics.where_clause);
        where_clause.predicates.push(if is_local {
            syn::parse_quote_spanned!(bound_span=> Self: 'minitrace)
        } else {
            syn::parse_quote_spanned!(bound_span=> Self: ::core::marker::#bound + 'minitrace)
        });
    }

    for (i, arg) in sig.inputs.iter_mut().enumerate() {
        match arg {
            syn::FnArg::Receiver(syn::Receiver {
                reference: Some(_), ..
            }) => {}
            syn::FnArg::Receiver(arg) => arg.mutability = None,
            syn::FnArg::Typed(arg) => {
                if let syn::Pat::Ident(ident) = &mut *arg.pat {
                    ident.by_ref = None;
                    //ident.mutability = None;
                } else {
                    let positional = positional_arg(i, &arg.pat);
                    let m = mut_pat(&mut arg.pat);
                    arg.pat = syn::parse_quote!(#m #positional);
                }
            }
        }
    }

    let ret_span = sig.ident.span();
    let bounds = if is_local {
        quote::quote_spanned!(ret_span=> 'minitrace)
    } else {
        quote::quote_spanned!(ret_span=> ::core::marker::Send + 'minitrace)
    };
    sig.output = syn::parse_quote_spanned! {ret_span=>
        -> impl ::core::future::Future<Output = #ret> + #bounds
    };
}

/// Generates an identifier for a positional argument.
///
/// # Examples
///
/// ```
/// // Assuming `pat` is a `syn::Pat` instance
/// let arg_ident = positional_arg(1, &pat);
/// ```
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Arguments
///
/// * `i` - The index of the argument.
/// * `pat` - The pattern of the argument.
///
/// # Notes
///
/// The `positional_arg` function is used to generate an identifier for a positional argument. It uses the `quote::format_ident!` macro to generate the identifier.
fn positional_arg(i: usize, pat: &syn::Pat) -> syn::Ident {
    quote::format_ident!("__arg{}", i, span = syn::spanned::Spanned::span(&pat))
}

/// Checks if a pattern is mutable.
///
/// # Examples
///
/// ```
/// // Assuming `pat` is a mutable `syn::Pat` instance
/// let is_mut = mut_pat(&mut pat);
/// ```
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Arguments
///
/// `pat` - The pattern to check.
///
/// # Notes
///
/// The `mut_pat` function is used to check if a pattern is mutable. It uses a visitor pattern to traverse the pattern and check for mutability.
fn mut_pat(pat: &mut syn::Pat) -> Option<syn::Token![mut]> {
    let mut visitor = HasMutPat(None);
    visitor.visit_pat_mut(pat);
    visitor.0
}

/// Checks if the `Self` keyword is present in a token stream.
///
/// # Examples
///
/// ```
/// // Assuming `tokens` is a `proc_macro2::TokenStream` instance
/// let has_self = has_self_in_token_stream(tokens);
/// ```
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Arguments
///
/// `tokens` - The token stream to check.
///
/// # Notes
///
/// The `has_self_in_token_stream` function is used to check if the `Self` keyword is present in a token stream. It uses a recursive approach to traverse the token stream.
fn has_self_in_token_stream(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|tt| match tt {
        proc_macro2::TokenTree::Ident(ident) => ident == "Self",
        proc_macro2::TokenTree::Group(group) => has_self_in_token_stream(group.stream()),
        _ => false,
    })
}

/// Returns the `WhereClause` of a function, or creates a new one if it doesn't exist.
///
/// # Examples
///
/// ```
/// // Assuming `clause` is a mutable reference to an `Option<syn::WhereClause>` instance
/// let where_clause = where_clause_or_default(&mut clause);
/// ```
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Arguments
///
/// `clause` - The `WhereClause` of the function.
///
/// # Notes
///
/// The `where_clause_or_default` function is used to get the `WhereClause` of a function, or create a new one if it doesn't exist. It uses the `Option::get_or_insert_with` method to achieve this.
fn where_clause_or_default(clause: &mut Option<syn::WhereClause>) -> &mut syn::WhereClause {
    clause.get_or_insert_with(|| syn::WhereClause {
        where_token: Default::default(),
        predicates: syn::punctuated::Punctuated::new(),
    })
}

struct HasMutPat(Option<syn::Token![mut]>);

impl syn::visit_mut::VisitMut for HasMutPat {
    fn visit_pat_ident_mut(&mut self, i: &mut syn::PatIdent) {
        if let Some(m) = i.mutability {
            self.0 = Some(m);
        } else {
            syn::visit_mut::visit_pat_ident_mut(self, i);
        }
    }
}

pub struct HasSelf(pub bool);

impl syn::visit_mut::VisitMut for HasSelf {
    /// Visits the `ExprPath` nodes in the syntax tree.
    ///
    /// This method is part of the `syn::visit_mut::VisitMut` trait implementation for `HasSelf`. It checks if the first segment of the path is `Self` and updates the state accordingly.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `visitor` is a `HasSelf` instance and `expr` is a `syn::ExprPath` instance
    /// visitor.visit_expr_path_mut(&mut expr);
    /// ```
    ///
    /// # Safety
    ///
    /// This method does not use any unsafe code.
    ///
    /// # Arguments
    ///
    /// `expr` - The `ExprPath` node to visit.
    ///
    /// # Notes
    ///
    /// The `visit_expr_path_mut` method is part of the `syn::visit_mut::VisitMut` trait implementation for `HasSelf`. It checks if the first segment of the path is `Self` and updates the state accordingly.
    fn visit_expr_path_mut(&mut self, expr: &mut syn::ExprPath) {
        self.0 |= expr.path.segments[0].ident == "Self";
        syn::visit_mut::visit_expr_path_mut(self, expr);
    }

    /// Visits the `PatPath` nodes in the syntax tree.
    ///
    /// This method is part of the `syn::visit_mut::VisitMut` trait implementation for `HasSelf`. It checks if the first segment of the path is `Self` and updates the state accordingly.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `visitor` is a `HasSelf` instance and `pat` is a `syn::PatPath` instance
    /// visitor.visit_pat_path_mut(&mut pat);
    /// ```
    ///
    /// # Safety
    ///
    /// This method does not use any unsafe code.
    ///
    /// # Arguments
    ///
    /// `pat` - The `PatPath` node to visit.
    ///
    /// # Notes
    ///
    /// The `visit_pat_path_mut` method is part of the `syn::visit_mut::VisitMut` trait implementation for `HasSelf`. It checks if the first segment of the path is `Self` and updates the state accordingly.
    fn visit_pat_path_mut(&mut self, pat: &mut syn::PatPath) {
        self.0 |= pat.path.segments[0].ident == "Self";
        syn::visit_mut::visit_pat_path_mut(self, pat);
    }

    /// Visits the `TypePath` nodes in the syntax tree.
    ///
    /// This method is part of the `syn::visit_mut::VisitMut` trait implementation for `HasSelf`. It checks if the first segment of the path is `Self` and updates the state accordingly.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `visitor` is a `HasSelf` instance and `ty` is a `syn::TypePath` instance
    /// visitor.visit_type_path_mut(&mut ty);
    /// ```
    ///
    /// # Safety
    ///
    /// This method does not use any unsafe code.
    ///
    /// # Arguments
    ///
    /// `ty` - The `TypePath` node to visit.
    ///
    /// # Notes
    ///
    /// The `visit_type_path_mut` method is part of the `syn::visit_mut::VisitMut` trait implementation for `HasSelf`. It checks if the first segment of the path is `Self` and updates the state accordingly.
    fn visit_type_path_mut(&mut self, ty: &mut syn::TypePath) {
        self.0 |= ty.path.segments[0].ident == "Self";
        syn::visit_mut::visit_type_path_mut(self, ty);
    }

    /// Visits the `Receiver` nodes in the syntax tree.
    ///
    /// This method is part of the `syn::visit_mut::VisitMut` trait implementation for `HasSelf`. It sets the state to `true` when a `Receiver` node is visited.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `visitor` is a `HasSelf` instance and `arg` is a `syn::Receiver` instance
    /// visitor.visit_receiver_mut(&mut arg);
    /// ```
    ///
    /// # Safety
    ///
    /// This method does not use any unsafe code.
    ///
    /// # Arguments
    ///
    /// `arg` - The `Receiver` node to visit.
    ///
    /// # Notes
    ///
    /// The `visit_receiver_mut` method is part of the `syn::visit_mut::VisitMut` trait implementation for `HasSelf`. It sets the state to `true` when a `Receiver` node is visited.
    fn visit_receiver_mut(&mut self, _arg: &mut syn::Receiver) {
        self.0 = true;
    }

    /// Visits the `Item` nodes in the syntax tree.
    ///
    /// This method is part of the `syn::visit_mut::VisitMut` trait implementation for `HasSelf`. It does not recurse into nested items.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `visitor` is a `HasSelf` instance and `item` is a `syn::Item` instance
    /// visitor.visit_item_mut(&mut item);
    /// ```
    ///
    /// # Safety
    ///
    /// This method does not use any unsafe code.
    ///
    /// # Arguments
    ///
    /// `item` - The `Item` node to visit.
    ///
    /// # Notes
    ///
    /// The `visit_item_mut` method is part of the `syn::visit_mut::VisitMut` trait implementation for `HasSelf`. It does not recurse into nested items.
    fn visit_item_mut(&mut self, _: &mut syn::Item) {
        // Do not recurse into nested items.
    }

    /// Visits the `Macro` nodes in the syntax tree.
    ///
    /// This method is part of the `syn::visit_mut::VisitMut` trait implementation for `HasSelf`. It checks if the macro contains a function and updates the state accordingly.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `visitor` is a `HasSelf` instance and `mac` is a `syn::Macro` instance
    /// visitor.visit_macro_mut(&mut mac);
    /// ```
    ///
    /// # Safety
    ///
    /// This method does not use any unsafe code.
    ///
    /// # Arguments
    ///
    /// `mac` - The `Macro` node to visit.
    ///
    /// # Notes
    ///
    /// The `visit_macro_mut` method is part of the `syn::visit_mut::VisitMut` trait implementation for `HasSelf`. It checks if the macro contains a function and updates the state accordingly.
    fn visit_macro_mut(&mut self, mac: &mut syn::Macro) {
        if !contains_fn(mac.tokens.clone()) {
            self.0 |= has_self_in_token_stream(mac.tokens.clone());
        }
    }
}

/// Checks if the `fn` keyword is present in a token stream.
///
/// This function is used to check if the `fn` keyword is present in a token stream. It uses a recursive approach to traverse the token stream.
///
/// # Examples
///
/// ```
/// // Assuming `tokens` is a `proc_macro2::TokenStream` instance
/// let contains_fn = contains_fn(tokens);
/// ```
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Arguments
///
/// `tokens` - The token stream to check.
///
/// # Notes
///
/// The `contains_fn` function is used to check if the `fn` keyword is present in a token stream. It uses a recursive approach to traverse the token stream.
fn contains_fn(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|tt| match tt {
        proc_macro2::TokenTree::Ident(ident) => ident == "fn",
        proc_macro2::TokenTree::Group(group) => contains_fn(group.stream()),
        _ => false,
    })
}
