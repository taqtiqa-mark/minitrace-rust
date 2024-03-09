/// Implementation Notes:
///
/// Check for `async-trait`-like patterns in the block, and instrument the
/// future instead of the wrapper.
///
/// Instrumenting the `async fn` is not as straight forward as expected because
/// `async_trait` rewrites `async fn` into a normal `fn` which returns
/// `Pin<Box<dyn Future + Send + 'async_trait>>`, and this stops the macro from
/// distinguishing `async fn` from `fn`.
///
/// The following logic and code is from the `async-trait` probes from
/// [tokio-tracing][tokio-logic].
/// The Tokio logic is required for detecting the `async fn` that is already
/// transformed to `fn -> Pin<Box<dyn Future + Send + 'async_trait>>` by
/// `async-trait`.
/// We have to distinguish this case from `fn -> impl Future` that is written
/// by the user because for the latter, we instrument it like normal `fn`
/// instead of `async fn`.
///
/// The reason why we elaborate `async fn` into `fn -> impl Future`:
/// For an `async fn foo()`, we have to instrument the
/// `Span::enter_with_local_parent()` in the first call to `foo()`, but not in
/// the `poll()` or `.await`, because it's the only chance that
/// local parent is present in that context.
///
/// [tokio-logic]: https://github.com/tokio-rs/tracing/blob/6a61897a5e834988ad9ac709e28c93c4dbf29116/tracing-attributes/src/expand.rs

// Trace Attribute Features
//
// The feature set for the `trace` attribute is evolving, heading to a 1.0
// release.  The following features are under discussion.  Implementation
// will be non-trivial until issues #136 and issue #137 are resolved.
// A consequence of this is that implementation will need to be incremental
// rather than big-bang event.
//
// - `<Macro>.name: syn::LitStr,`
//       - See upstream issue #142
// - `<Macro>.enter_on_poll: syn::LitBool,`
//       - See upstream issue #133 and https://github.com/tikv/minitrace-rust/issues/126#issuecomment-1077326184
// - `<Macro>.parent: syn::LitStr,`
//       - See upstream issue #117
// - `<Macro>.recorder: syn::Ident,`
//       - See upstream issue #117
// - `<Macro>.recurse: syn::Ident,`
//       - See upstream issue #134
// - `<Macro>.scope: syn::Ident,`
//       - See upstream issue #133 and https://github.com/tikv/minitrace-rust/issues/126#issuecomment-1077326184
// - `<Macro>.variables: syn::Ident,`
//       - See upstream issue #134
// - `<Macro>.conventional: syn::LitBool,`
//       - Benefit is to short circuit some of the parsing logic and hopefully
//         save on compile time - conjecture.
//       - Assume & skip evaluations in analyze when `conventional=true`,
//         and follow these defaults/conventions:
//
//             - name: `fn` name (item). Including path(?)
//             - recorder: `span`
//             - recurse: `None`
//             - scope: `Local` (sync), `Local` (async).
//             - variables: `None`
//             - enter_on_poll:
//               - `None` (sync)
//               - `true` (async) if `false` then convention is that scope: `Threads`.
//
//   Note: These conventions change the current defaults.
//         See https://github.com/tikv/minitrace-rust/issues/126#issuecomment-1077326184
//
//   Current default:
//
//   - `#[trace] async fn` creates thread-safe span (`Span`)
//         - `#[trace(enter_on_poll = true)] async fn` creates local context
//           span (`LocalSpan`)
//   - `#[trace] fn` create local context span (`LocalSpan`)
//
// impl Default for Model {
//
//     fn default() -> Self {
//         Ok(Model {
//             name: todo!(),
//             enter_on_poll: todo!(),
//             parent: todo!(),
//             recorder: todo!(),
//             scope: todo!(),
//             variables: v,
//         })
// }

#[derive(Clone, Copy, Debug, PartialEq, darling::FromMeta)]
pub enum Scope {
    Local,
    Threads,
}

// `Trace` should be moved into `minitrace-macro::validate`.
// Implement `syn::Parse` there, so that in `lib.rs`:
//
//    let attr_args = parse_macro_input!(argsc as crate::trace::validate::TraceAttr);
//    let itemfn = parse_macro_input!(itemc as ItemFn);
//    let args2: proc_macro2::TokenStream = args.clone().into();
//    trace::validate(args2, item.into());
//    let model = trace::analyze(attr_args, itemfn);
//
// becomes
//
//    use crate::trace::validate::Trace;
//    let trace = parse_macro_input!(argsc as Trace);
//    let item = parse_macro_input!(itemc as Trace);
//    let model = trace::analyze(trace, item);
#[derive(
    Clone,
    std::fmt::Debug,
    PartialEq,
    // `darling::FromMeta,` adds two functions:
    //
    // ```
    // fn from_list(items: &[NestedMeta]) -> Result<Trace, syn::Error>
    // ```
    //
    // `try_from_attributes(...)` returns:
    //   - `Ok(None)` if the attribute is missing,
    //   - `Ok(Some(_))` if its there and is valid,
    //   - `Err(_)` otherwise.
    darling::FromMeta,
)]

/// A struct representing the parsed attributes of the `#[trace]` macro.
///
/// # Arguments
///
/// * `name`: The name of the span. Defaults to `None` if not specified.
/// * `scope`: The scope of the span. Can be `Scope::Local`, `Scope::Thread`, etc. Defaults to `None` if not specified.
/// * `enter_on_poll`: If `true`, the span will be entered on each poll of the future. Defaults to `None` if not specified.
/// * `parent`: The name of the parent span. Defaults to `None` if not specified.
/// * `recorder`: The recorder to use for this span. Defaults to `None` if not specified.
/// * `recurse`: If `true`, the span will be applied recursively to all called functions. Defaults to `None` if not specified.
/// * `root`: If `true`, the span will be a root span. Defaults to `None` if not specified.
/// * `variables`: An array of variables to be recorded. Defaults to `None` if not specified.
/// * `async_trait`: If `true`, the span will be applied to async trait methods. Defaults to `None` if not specified.
///
/// # Examples
///
/// ```
/// #[trace(name = "foo", scope = Scope::Thread, enter_on_poll = true,
///         parent = "bar", recorder = "baz", recurse = true, root = true,
///         variables = ["x", "y", "z"], async_trait = true)]
/// fn foo() {
///     // ...
/// }
/// ```
///
/// # Safety
///
/// This struct does not use any unsafe code.
///
/// # Panics
///
/// This struct does not panic under normal conditions. However, it may panic if there's an issue with the underlying `syn` implementation.
pub struct Trace {
    // Anything that implements `syn::parse::Parse` is supported.
    #[darling(default)]
    name: Option<syn::LitStr>,
    #[darling(default)]
    scope: Option<Scope>, // Scope::Local, Scope::Thread, etc.

    // Fields wrapped in `Option` are and default to `None` if
    // not specified in the attribute.
    #[darling(default)]
    enter_on_poll: Option<syn::LitBool>,
    #[darling(default)]
    parent: Option<syn::LitStr>,
    #[darling(default)]
    recorder: Option<syn::Ident>,
    #[darling(default)]
    recurse: Option<syn::LitBool>,
    #[darling(default)]
    root: Option<syn::LitBool>,
    #[darling(default)]
    variables: Option<syn::ExprArray>,
    #[darling(default)]
    async_trait: Option<syn::LitBool>,
}

/// Analyzes the provided `Trace` and `TokenStream` and produces a `Models` object.
///
/// This function visits each function in the `TokenStream` and merges it with its trace settings.
/// The trace settings are determined by the `Trace` object.
///
/// # Arguments
///
/// * `trace`: A `Trace` object that holds the attribute parameters.
/// * `items`: A `TokenStream` that represents the items the `#[trace]` attribute is applied to.
///
/// # Examples
///
/// ```
/// let trace = Trace {
///     name: Some(syn::LitStr::new("foo", proc_macro2::Span::call_site())),
///     scope: Some(Scope::Thread),
///     enter_on_poll: Some(syn::LitBool::new(true, proc_macro2::Span::call_site())),
///     parent: Some(syn::LitStr::new("bar", proc_macro2::Span::call_site())),
///     recorder: Some(syn::Ident::new("baz", proc_macro2::Span::call_site())),
///     recurse: Some(syn::LitBool::new(true, proc_macro2::Span::call_site())),
///     root: Some(syn::LitBool::new(true, proc_macro2::Span::call_site())),
///     variables: Some(syn::ExprArray {
///         attrs: Vec::new(),
///         bracket_token: syn::token::Bracket(proc_macro2::Span::call_site()),
///         elems: Punctuated::new(),
///     }),
///     async_trait: Some(syn::LitBool::new(true, proc_macro2::Span::call_site())),
/// };
///
/// let items = quote! {
///     fn foo() {
///         // ...
///     }
/// };
///
/// let models = analyze(trace, items.into());
/// ```
///
/// # Panics
///
/// This function will panic if the provided `TokenStream` cannot be parsed into a `syn::File`.
use syn::visit::Visit;
pub fn analyze(
    //args: std::vec::Vec<syn::NestedMeta>,
    trace: crate::trace::Trace,
    items: proc_macro2::TokenStream,
) -> Models<Model> {
    let mut models = Models::<Model>::new();

    // Prepare and merge each ItemFn with its trace settings
    let tree: syn::File = syn::parse2(items).unwrap();
    let mut visitor = FnVisitor {
        functions: Vec::new(),
    };
    visitor.visit_file(&tree);
    for f in visitor.functions {
        let item_fn = (*f).clone();
        let default_name = item_fn.sig.ident.to_string();
        let _async_fn = match item_fn.sig.asyncness {
            Some(_) => Some(syn::LitBool::new(true, proc_macro2::Span::call_site())),
            None => Some(syn::LitBool::new(false, proc_macro2::Span::call_site())),
        };
        let traced_item = if let crate::trace::Trace {
            default: _,
            validated: _,
            name,
            scope: Some(scope),
            enter_on_poll,
            parent: Some(parent),
            recorder: Some(recorder),
            recurse: Some(recurse),
            root: Some(root),
            variables: Some(variables),
            async_trait: Some(async_trait),
            async_fn: Some(async_fn),
        } = trace.clone()
        {
            // Use default name when no name is passed in.
            // NOTE:
            //     `#[trace(key = "value")]` maps to
            //     `#[trace(name = "__default", key = "value")]`
            let span_name = if name.value() == "__default" {
                syn::LitStr::new(&default_name, proc_macro2::Span::call_site())
            } else {
                name
            };

            TracedItem {
                name: span_name,
                scope,
                enter_on_poll,
                parent,
                recorder,
                recurse,
                root,
                variables,
                async_trait,
                async_fn,
                item_fn,
            }
        } else {
            TracedItem {
                ..Default::default()
            }
        };
        models.push(Model::Item(Box::new(traced_item)));
    }
    models
}

/// A newtype wrapper around `Vec<T>` that allows for the implementation of any trait.
///
/// This struct is used to circumvent the orphan rule and provides flexibility in terms of encapsulating or exposing
/// Vector functionality as required.
///
/// The `From` trait provides conveniences for handling errors, such as transforming them into compile errors.
///
/// The following traits are implemented for this struct:
///
/// - `Debug` (via `#[derive(...)]`)
/// - `Default`
/// - `Deref`
/// - `DerefMut`
/// - `Display`
///
/// # Examples
///
/// ```
/// let models: Models<i32> = Models::new();
/// models.push(1);
/// models.push(2);
/// models.push(3);
/// assert_eq!(models.len(), 3);
/// ```
///
/// # Safety
///
/// This struct does not use any unsafe code.
///
/// # Panics
///
/// This struct does not panic under normal conditions. However, it may panic if there's an issue with the underlying `Vec` implementation.
#[derive(Debug, Clone, PartialEq)]
pub struct Models<T>(Vec<T>);

/// Implementation of `Models` struct.
impl<T: std::fmt::Debug> Models<T> {
    /// Creates a new `Models` object.
    ///
    /// # Examples
    ///
    /// ```
    /// let models: Models<i32> = Models::new();
    /// assert_eq!(models.len(), 0);
    /// ```
    ///
    /// # Panics
    ///
    /// This function does not panic under normal conditions.
    pub fn new() -> Models<T> {
        Models(Vec::<T>::new())
    }

    /// Creates a new `Models` object with the specified capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity`: The capacity for the new `Models` object.
    ///
    /// # Examples
    ///
    /// ```
    /// let models: Models<i32> = Models::with_capacity(10);
    /// assert_eq!(models.capacity(), 10);
    /// ```
    ///
    /// # Panics
    ///
    /// This function will panic if the specified capacity causes the system to run out of memory.
    #[allow(dead_code)]
    pub fn with_capacity(capacity: usize) -> Models<T> {
        Models(Vec::<T>::with_capacity(capacity))
    }
}

/// Provides a `Default` implementation for `Models`.
///
/// This implementation calls `Models::new()`, which creates an empty `Models` object.
///
/// # Examples
///
/// ```
/// let models: Models<i32> = Default::default();
/// assert_eq!(models.len(), 0);
/// ```
///
/// # Panics
///
/// This function does not panic under normal conditions.
impl<T: std::fmt::Debug> Default for Models<T> {
    fn default() -> Models<T> {
        Models::new()
    }
}

/// Provides a `Display` implementation for `Models`.
///
/// This implementation uses the `Debug` implementation of the inner type `T` to format the `Models` object.
///
/// # Examples
///
/// ```
/// let mut models: Models<i32> = Models::new();
/// models.push(1);
/// models.push(2);
/// models.push(3);
/// assert_eq!(format!("{}", models), "[1, 2, 3]");
/// ```
///
/// # Errors
///
/// This function will return an `Err` if the formatter encounters any errors.
impl<T: std::fmt::Debug> std::fmt::Display for Models<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// Provides a `Deref` implementation for `Models`.
///
/// This implementation allows `Models` to be used wherever `Vec<T>` is expected.
///
/// # Examples
///
/// ```
/// let mut models: Models<i32> = Models::new();
/// models.push(1);
/// models.push(2);
/// models.push(3);
/// assert_eq!(models.len(), 3);
/// assert_eq!(models[0], 1);
/// assert_eq!(models[1], 2);
/// assert_eq!(models[2], 3);
/// ```
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Panics
///
/// This function does not panic under normal conditions. However, it may panic if there's an issue with the underlying `Vec` implementation.
impl<T: std::fmt::Debug> std::ops::Deref for Models<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Vec<T> {
        &self.0
    }
}

/// Provides a `DerefMut` implementation for `Models`.
///
/// This implementation allows mutable references to `Models` to be used wherever mutable references to `Vec<T>` are expected.
///
/// # Examples
///
/// ```
/// let mut models: Models<i32> = Models::new();
/// models.push(1);
/// models.push(2);
/// models.push(3);
/// models[0] = 4;
/// models[1] = 5;
/// models[2] = 6;
/// assert_eq!(models[0], 4);
/// assert_eq!(models[1], 5);
/// assert_eq!(models[2], 6);
/// ```
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Panics
///
/// This function does not panic under normal conditions. However, it may panic if there's an issue with the underlying `Vec` implementation.
impl<T: std::fmt::Debug> std::ops::DerefMut for Models<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// `TracedItem` is a struct that holds the parsed attributes and the function to be traced.
///
/// This struct is used to hold the parsed attributes from the `#[trace(...)]` macro and the function that the macro is applied to.
/// The attributes are used to control the behavior of the tracing macro.
///
/// # Examples
///
/// ```
/// // Assuming `fn_to_trace` is a syn::ItemFn and `attrs` is a syn::AttributeArgs
/// let traced_item = TracedItem {
///     name: syn::LitStr::new("fn_to_trace", proc_macro2::Span::call_site()),
///     scope: crate::trace::parse::Scope::Thread,
///     enter_on_poll: syn::LitBool::new(true, proc_macro2::Span::call_site()),
///     parent: syn::LitStr::new("", proc_macro2::Span::call_site()),
///     recorder: syn::Ident::new("recorder", proc_macro2::Span::call_site()),
///     recurse: syn::LitBool::new(false, proc_macro2::Span::call_site()),
///     root: syn::LitBool::new(false, proc_macro2::Span::call_site()),
///     variables: syn::ExprArray {
///         attrs: attrs,
///         bracket_token: syn::token::Bracket(proc_macro2::Span::call_site()),
///         elems: proc_macro2::Punctuated::new(),
///     },
///     async_trait: syn::LitBool::new(false, proc_macro2::Span::call_site()),
///     async_fn: syn::LitBool::new(true, proc_macro2::Span::call_site()),
///     item_fn: fn_to_trace,
/// };
/// ```
///
/// # Safety
///
/// This struct does not use any unsafe code.
#[derive(Clone, Debug, PartialEq)]
pub struct TracedItem {
    // These are the fields parsed as AttributeArgs into the `Trace` struct
    pub name: syn::LitStr,
    pub scope: crate::trace::parse::Scope, // Scope::Local, Scope::Thread, etc.
    pub enter_on_poll: syn::LitBool,
    pub parent: syn::LitStr,
    pub recorder: syn::Ident,
    pub recurse: syn::LitBool,
    pub root: syn::LitBool,
    pub variables: syn::ExprArray,
    pub async_trait: syn::LitBool,
    pub async_fn: syn::LitBool,

    // `item_fn` pairs each function with the `#[trace(...)]` settings.
    // This structure admits the `recurse=true` option contemplated in issue #134
    pub item_fn: syn::ItemFn,
}

/// `Model` is an enum that represents the different types of models that can be analyzed.
///
/// This enum is used to represent either an `Attribute` or an `Item`. The `Attribute` variant holds a `Trace` object,
/// while the `Item` variant holds a `TracedItem` object. The `Item` variant is boxed to satisfy the
/// `clippy::large-enum-variant` lint which is triggered by CI settings.
///
/// # Examples
///
/// ```
/// // Assuming `trace` is a Trace object and `traced_item` is a TracedItem object
/// let model_attribute = Model::Attribute(trace);
/// let model_item = Model::Item(Box::new(traced_item));
/// ```
///
/// # Errors
///
/// This enum has a `thiserror::Error` derive, and it will return "Validation logic error" if it's used as an error.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[error("Validation logic error")]
pub enum Model {
    Attribute(Trace),
    // Boxed to satisfy clippy::large-enum-variant which is triggered by CI settings
    Item(Box<TracedItem>),
}

/// `FnVisitor` is a struct used to populate `Models` (a Vec-newtype) when
/// `#[trace(recurse=all|public|private)]` is applied on a function or, eventually,
/// a module.
///
/// This struct holds a vector of references to `syn::ItemFn` objects, which represent the functions
/// that are being visited.
///
/// # Examples
///
/// ```
/// // Assuming `item_fns` is a Vec<&syn::ItemFn>
/// let fn_visitor = FnVisitor {
///     functions: item_fns,
/// };
/// ```
///
/// # Lifetimes
///
/// `'ast` - Represents the lifetime of the abstract syntax tree. This is the lifetime of the references to the `syn::ItemFn` objects.
///
/// # Arguments
///
/// `functions` - A vector of references to `syn::ItemFn` objects. These represent the functions that are being visited.
struct FnVisitor<'ast> {
    functions: Vec<&'ast syn::ItemFn>,
}

/// Visits a function item in the syntax tree.
///
/// This method is part of the `syn::visit::Visit` trait implementation for `FnVisitor`. It is called when a function item is encountered in the syntax tree. The function item is added to the `functions` vector of the `FnVisitor`.
///
/// # Examples
///
/// ```
/// // Assuming `fn_visitor` is a FnVisitor and `item_fn` is a syn::ItemFn
/// fn_visitor.visit_item_fn(&item_fn);
/// assert!(fn_visitor.functions.contains(&item_fn));
/// ```
///
/// # Lifetimes
///
/// `'ast` - Represents the lifetime of the abstract syntax tree. This is the lifetime of the references to the `syn::ItemFn` objects.
///
/// # Arguments
///
/// `node` - A reference to the function item that is being visited.
impl<'ast> syn::visit::Visit<'ast> for FnVisitor<'ast> {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.functions.push(node);
        // Delegate to the default impl to visit any nested functions.
        syn::visit::visit_item_fn(self, node);
    }
}

/// Provides a `From<proc_macro2::TokenStream>` implementation for `Model`.
///
/// This implementation allows a `proc_macro2::TokenStream` to be converted into a `Model::Attribute` with default values.
///
/// # Examples
///
/// ```
/// // Assuming `token_stream` is a proc_macro2::TokenStream
/// let model: Model = token_stream.into();
/// match model {
///     Model::Attribute(attribute) => assert_eq!(attribute, Trace::default()),
///     _ => panic!("Expected Model::Attribute"),
/// }
/// ```
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Panics
///
/// This function does not panic under normal conditions. However, it may panic if there's an issue with the `Default` implementation for `Trace`.
///
/// # Arguments
///
/// `_inner` - The `proc_macro2::TokenStream` that is being converted into a `Model`. This argument is currently unused.
impl std::convert::From<proc_macro2::TokenStream> for Model {
    fn from(_inner: proc_macro2::TokenStream) -> Model {
        let attribute = Default::default();
        Model::Attribute(attribute)
    }
}

/// Provides a `From<proc_macro2::TokenStream>` implementation for `Models<Model>`.
///
/// This implementation allows a `proc_macro2::TokenStream` to be converted into a `Models<Model>` with a single `Model::Attribute` with default values.
///
/// # Examples
///
/// ```
/// // Assuming `token_stream` is a proc_macro2::TokenStream
/// let models: Models<Model> = token_stream.into();
/// assert_eq!(models.len(), 1);
/// match &models[0] {
///     Model::Attribute(attribute) => assert_eq!(attribute, &Trace::default()),
///     _ => panic!("Expected Model::Attribute"),
/// }
/// ```
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Panics
///
/// This function does not panic under normal conditions. However, it may panic if there's an issue with the `Default` implementation for `Trace`.
///
/// # Arguments
///
/// `_inner` - The `proc_macro2::TokenStream` that is being converted into a `Models<Model>`. This argument is currently unused.
impl std::convert::From<proc_macro2::TokenStream> for Models<Model> {
    fn from(_inner: proc_macro2::TokenStream) -> Models<Model> {
        let attribute = Default::default();
        let mut models = Models::<Model>::new();
        models.push(Model::Attribute(attribute));
        models
    }
}

/// Provides a `Default` implementation for `Trace`.
///
/// This implementation creates a `Trace` object with default values. The `name`, `scope`, `enter_on_poll`, `recorder`, `recurse`, `root`, `variables`, `parent`, and `async_trait` fields are all set to some default value.
///
/// # Examples
///
/// ```
/// let trace = Trace::default();
/// assert_eq!(trace.name, Some(syn::LitStr::new("__default", proc_macro2::Span::call_site())));
/// assert_eq!(trace.scope, Some(Scope::Local));
/// assert_eq!(trace.enter_on_poll, Some(syn::LitBool::new(false, proc_macro2::Span::call_site())));
/// // ... and so on for the other fields
/// ```
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Panics
///
/// This function does not panic under normal conditions. However, it may panic if there's an issue with the `new` function for `syn::LitStr`, `syn::LitBool`, or `syn::Ident`.
impl Default for Trace {
    fn default() -> Self {
        // let scope = proc_macro2::Ident::new("Local", proc_macro2::Span::call_site());
        // Some(syn::LitBool::new(false, proc_macro2::Span::call_site()));
        let name = Some(syn::LitStr::new(
            "__default",
            proc_macro2::Span::call_site(),
        ));
        let scope = Some(Scope::Local);
        let enter_on_poll = Some(syn::LitBool::new(false, proc_macro2::Span::call_site()));
        let recorder = Some(proc_macro2::Ident::new(
            "span",
            proc_macro2::Span::call_site(),
        ));
        let recurse = Some(syn::LitBool::new(false, proc_macro2::Span::call_site()));
        let root = Some(syn::LitBool::new(false, proc_macro2::Span::call_site()));
        let variables = Some(syn::parse_quote!([]));
        let parent = Some(syn::LitStr::new(
            "__default",
            proc_macro2::Span::call_site(),
        ));
        let async_trait = Some(syn::LitBool::new(false, proc_macro2::Span::call_site()));

        Self {
            name,
            async_trait,
            enter_on_poll,
            parent,
            recorder,
            recurse,
            root,
            scope,
            variables,
        }
    }
}

/// Provides a `Default` implementation for `TracedItem`.
///
/// This implementation creates a `TracedItem` object with default values. The `name`, `scope`, `enter_on_poll`, `item_fn`, `recorder`, `recurse`, `root`, `variables`, `parent`, `async_trait`, and `async_fn` fields are all set to some default value.
///
/// # Examples
///
/// ```
/// let traced_item = TracedItem::default();
/// assert_eq!(traced_item.name.value(), "__default");
/// assert_eq!(traced_item.scope, crate::trace::parse::Scope::Local);
/// assert_eq!(traced_item.enter_on_poll.value(), false);
/// // ... and so on for the other fields
/// ```
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Panics
///
/// This function does not panic under normal conditions. However, it may panic if there's an issue with the `new` function for `syn::LitStr`, `syn::LitBool`, or `syn::Ident`.
impl Default for TracedItem {
    fn default() -> Self {
        // let scope = proc_macro2::Ident::new("Local", proc_macro2::Span::call_site());
        // Some(syn::LitBool::new(false, proc_macro2::Span::call_site()));
        let name = syn::LitStr::new("__default", proc_macro2::Span::call_site());
        let scope = crate::trace::parse::Scope::Local;
        let enter_on_poll = syn::LitBool::new(false, proc_macro2::Span::call_site());
        let item_fn: syn::ItemFn = syn::parse_quote!(
            fn __default() {}
        );
        let recorder = proc_macro2::Ident::new("span", proc_macro2::Span::call_site());
        let recurse = syn::LitBool::new(false, proc_macro2::Span::call_site());
        let root = syn::LitBool::new(false, proc_macro2::Span::call_site());
        let variables = syn::parse_quote!([]);
        let parent = syn::LitStr::new("__default", proc_macro2::Span::call_site());
        let async_trait = syn::LitBool::new(false, proc_macro2::Span::call_site());
        let async_fn = syn::LitBool::new(false, proc_macro2::Span::call_site());

        Self {
            name,
            async_trait,
            async_fn,
            enter_on_poll,
            item_fn,
            parent,
            recorder,
            recurse,
            root,
            scope,
            variables,
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::Attribute;

    use super::*;

    use crate::trace::analyze::Model;
    use crate::trace::analyze::Models;

    #[test]
    fn models_are_cloneable() {
        let models = Models::<Model>::new();
        let clones = models.clone();
        assert_eq!(models, clones);
    }
    #[test]
    fn with_traces() {
        // `#[trace]`
        //let args: Vec<syn::NestedMeta> = vec![];
        let trace = crate::trace::Trace {
            ..Default::default()
        };

        let items: proc_macro2::TokenStream = syn::parse_quote!(
            #[trace]
            fn f(x: bool) {}
        );
        let models = analyze(trace, items.clone());

        let model = (*models.get(0).unwrap()).clone();
        let traced_item = if let Model::Item(ti) = model {
            Ok((*ti).clone())
        } else {
            Err(())
        }
        .unwrap();
        let expected = TracedItem {
            name: syn::LitStr::new("f", proc_macro2::Span::call_site()),
            item_fn: syn::parse2::<syn::ItemFn>(items).unwrap(),
            ..Default::default()
        };
        assert_eq!(traced_item, expected);
    }

    #[test]
    fn with_trace() {
        // `#[trace]`
        //let args: Vec<syn::NestedMeta> = vec![];
        let trace = crate::trace::Trace {
            ..Default::default()
        };

        let items: proc_macro2::TokenStream = syn::parse_quote!(
            fn f(x: bool) {}
        );
        let models = analyze(trace, items.clone());

        let model = (*models.get(0).unwrap()).clone();
        let traced_item = if let Model::Item(ti) = model {
            Ok((*ti).clone())
        } else {
            Err(())
        }
        .unwrap();
        let expected = TracedItem {
            name: syn::LitStr::new("f", proc_macro2::Span::call_site()),
            item_fn: syn::parse2::<syn::ItemFn>(items).unwrap(),
            ..Default::default()
        };
        assert_eq!(traced_item, expected);
    }

    // There is no filtering/validation in the `analyze` function.
    // All such checks are done in `validate` function.
    #[test]
    fn others_with_traces() {
        // `#[trace]`
        //let args: Vec<syn::NestedMeta> = vec![];
        let trace = crate::trace::Trace {
            ..Default::default()
        };
        let models = analyze(
            trace,
            quote::quote!(
                #[a]
                #[trace]
                #[b]
                fn f(x: bool) -> bool {
                    x
                }
            ),
        );
        let expected: &[Attribute] = &[
            syn::parse_quote!(#[a]),
            syn::parse_quote!(#[trace]),
            syn::parse_quote!(#[b]),
        ];
        let model = (*models.get(0).unwrap()).clone();
        let traced_item = if let Model::Item(item) = model {
            *item.clone()
        } else {
            return;
        };
        let TracedItem { item_fn, .. } = traced_item;
        assert_eq!(expected, item_fn.attrs);
    }

    #[test]
    fn others_with_no_trace() {
        // `#[trace]`
        //let args: Vec<syn::NestedMeta> = vec![];
        let trace = crate::trace::Trace {
            ..Default::default()
        };

        let models = analyze(
            trace,
            syn::parse_quote!(
                #[a]
                #[b]
                fn f(x: bool) {}
            ),
        );
        let expected: &[Attribute] = &[syn::parse_quote!(#[a]), syn::parse_quote!(#[b])];
        let model = (*models.get(0).unwrap()).clone();
        let traced_item = if let Model::Item(item) = model {
            *item.clone()
        } else {
            return;
        };
        let TracedItem { item_fn, .. } = traced_item;
        assert_eq!(expected, item_fn.attrs);
    }
}
