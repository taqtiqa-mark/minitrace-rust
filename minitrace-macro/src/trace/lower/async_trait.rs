/// Represents the kind of an async trait.
///
/// The `AsyncTraitKind` enum has two variants: `Function` and `Async`. `Function` represents an async trait that is a function, while `Async` represents an async trait that is an async block.
///
/// # Examples
///
/// ```
/// // Assuming `item_fn` is a syn::ItemFn for the function `async fn foo()`
/// let function_kind = AsyncTraitKind::Function(&item_fn);
/// assert_eq!(function_kind, AsyncTraitKind::Function(&item_fn));
///
/// // Assuming `expr_async` is a syn::ExprAsync for the async block `async {}`
/// let async_kind = AsyncTraitKind::Async(&expr_async);
/// assert_eq!(async_kind, AsyncTraitKind::Async(&expr_async));
/// ```
///
/// # Safety
///
/// This enum does not use any unsafe code.
///
/// # Panics
///
/// This enum does not panic under normal conditions.
///
/// # Lifetimes
///
/// `'a` - The lifetime of the references to the `syn::ItemFn` and `syn::ExprAsync` in the `Function` and `Async` variants respectively.
pub enum AsyncTraitKind<'a> {
    // old construction. Contains the function
    Function(&'a syn::ItemFn),
    // new construction. Contains a reference to the async block
    Async(&'a syn::ExprAsync),
}

/// Represents information about an async trait.
///
/// The `AsyncTraitInfo` struct contains a reference to the source statement that must be patched and the kind of the async trait.
///
/// # Examples
///
/// ```
/// // Assuming `stmt` is a syn::Stmt for the statement `let x = 5;`
/// // and `kind` is an AsyncTraitKind::Function for the function `async fn foo()`
/// let info = AsyncTraitInfo {
///     _source_stmt: &stmt,
///     kind: kind,
/// };
/// assert_eq!(info._source_stmt, &stmt);
/// assert_eq!(info.kind, kind);
/// ```
///
/// # Safety
///
/// This struct does not use any unsafe code.
///
/// # Panics
///
/// This struct does not panic under normal conditions.
///
/// # Lifetimes
///
/// `'a` - The lifetime of the references to the `syn::Stmt` and the `AsyncTraitKind` in the `_source_stmt` and `kind` fields respectively.
pub struct AsyncTraitInfo<'a> {
    // statement that must be patched
    _source_stmt: &'a syn::Stmt,
    pub kind: AsyncTraitKind<'a>,
}

/// Extracts information about an async trait from a given block if it was generated
/// by async-trait.
///
/// This function inspects the given block to determine if it matches the pattern of an async trait.
/// If it does, it returns the statement that must be instrumented, along with some other information.
///
/// # Examples
///
/// ```
/// // Assuming `block` is a syn::Block for the block `{ async fn foo() {...}; Box::pin(foo()) }`
/// let info = get_async_trait_info(&block, false);
/// assert!(info.is_some());
/// ```
///
/// # Errors
///
/// This function does not return any errors. If the block does not represent an async trait, it returns `None`.
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
/// `block` - A reference to a `syn::Block` that might represent an async trait.
///
/// `block_is_async` - A boolean indicating whether the block is async. If `true`, this function will return `None`.
///
/// # Notes
///
/// When we are given a function annotated by async-trait, that function
/// is only a placeholder that returns a pinned future containing the
/// user logic, and it is that pinned future that needs to be instrumented.
/// Were we to instrument its parent, we would only collect information
/// regarding the allocation of that future, and not its own span of execution.
/// Depending on the version of async-trait, we inspect the block of the function
/// to find if it matches the pattern
/// `async fn foo<...>(...) {...}; Box::pin(foo<...>(...))` (<=0.1.43), or if
/// it matches `Box::pin(async move { ... }) (>=0.1.44). We then return the
/// statement that must be instrumented, along with some other information.
/// 'gen_body' will then be able to use that information to instrument the
/// proper function/future.
///
/// This follows the approach suggested in
/// https://github.com/dtolnay/async-trait/issues/45#issuecomment-571245673
pub fn get_async_trait_info(
    block: &syn::Block,
    block_is_async: bool,
) -> Option<AsyncTraitInfo<'_>> {
    // are we in an async context? If yes, this isn't a async_trait-like pattern
    if block_is_async {
        return None;
    }

    // list of async functions declared inside the block
    let inside_funs = block.stmts.iter().filter_map(|stmt| {
        if let syn::Stmt::Item(syn::Item::Fn(fun)) = &stmt {
            // If the function is async, this is a candidate
            if fun.sig.asyncness.is_some() {
                return Some((stmt, fun));
            }
        }
        None
    });

    // last expression of the block (it determines the return value
    // of the block, so that if we are working on a function whose
    // `trait` or `impl` declaration is annotated by async_trait,
    // this is quite likely the point where the future is pinned)
    let (last_expr_stmt, last_expr) = block.stmts.iter().rev().find_map(|stmt| {
        if let syn::Stmt::Expr(expr) = stmt {
            Some((stmt, expr))
        } else {
            None
        }
    })?;

    // is the last expression a function call?
    let (outside_func, outside_args) = match last_expr {
        syn::Expr::Call(syn::ExprCall { func, args, .. }) => (func, args),
        _ => return None,
    };

    // is it a call to `Box::pin()`?
    let path = match outside_func.as_ref() {
        syn::Expr::Path(path) => &path.path,
        _ => return None,
    };
    if !path_to_string(path).ends_with("Box::pin") {
        return None;
    }

    // Does the call take an argument? If it doesn't,
    // it's not gonna compile anyway, but that's no reason
    // to (try to) perform an out of bounds access
    if outside_args.is_empty() {
        return None;
    }

    // Is the argument to Box::pin an async block that
    // captures its arguments?
    if let syn::Expr::Async(async_expr) = &outside_args[0] {
        // check that the move 'keyword' is present
        async_expr.capture?;

        return Some(AsyncTraitInfo {
            _source_stmt: last_expr_stmt,
            kind: AsyncTraitKind::Async(async_expr),
        });
    }

    // Is the argument to Box::pin a function call itself?
    let func = match &outside_args[0] {
        syn::Expr::Call(syn::ExprCall { func, .. }) => func,
        _ => return None,
    };

    // "stringify" the path of the function called
    let func_name = match **func {
        syn::Expr::Path(ref func_path) => path_to_string(&func_path.path),
        _ => return None,
    };

    // Was that function defined inside of the current block?
    // If so, retrieve the statement where it was declared and the function itself
    let (stmt_func_declaration, func) = inside_funs
        .into_iter()
        .find(|(_, fun)| fun.sig.ident == func_name)?;

    Some(AsyncTraitInfo {
        _source_stmt: stmt_func_declaration,
        kind: AsyncTraitKind::Function(func),
    })
}

/// Converts a `syn::Path` to a `String`.
///
/// This function iterates over the segments of the given path and concatenates them into a string,
/// using `::` as the separator. It uses a heuristic to prevent too many allocations.
///
/// # Examples
///
/// ```
/// // Assuming `path` is a syn::Path for the path `std::fmt::Write`
/// let path_str = path_to_string(&path);
/// assert_eq!(path_str, "std::fmt::Write");
/// ```
///
/// # Errors
///
/// This function does not return any errors. If writing to the string fails, it will panic.
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Panics
///
/// This function will panic if writing to the string fails.
///
/// # Lifetimes
///
/// `'a` - The lifetime of the reference to the `syn::Path` in the `path` argument.
///
/// # Arguments
///
/// `path` - A reference to a `syn::Path` that should be converted to a string.
///
/// # Notes
///
/// The function uses a heuristic to prevent too many allocations by initializing the result string
/// with a capacity based on the number of segments in the path.
fn path_to_string(path: &syn::Path) -> String {
    use std::fmt::Write;
    // some heuristic to prevent too many allocations
    let mut res = String::with_capacity(path.segments.len() * 5);
    for i in 0..path.segments.len() {
        write!(res, "{}", path.segments[i].ident).expect("writing to a String should never fail");
        if i < path.segments.len() - 1 {
            res.push_str("::");
        }
    }
    res
}
