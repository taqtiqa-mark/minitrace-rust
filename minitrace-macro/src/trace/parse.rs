// Parse TokenStream
//
// Parse attribute arguments, which arrive as a `proc_macro::TokenStream`,
// into a `Vector` of `syn::NestedMeta` items.
//
// The input stream comes from the `trace::validate::validate` function.
// The output vector goes to the `trace::analyze::analyze` function.

/// Represents the scope of a traced item.
///
/// The `Scope` enum has two variants: `Local` and `Threads`. `Local` represents a traced item that is local to a function or method, while `Threads` represents a traced item that is shared across threads.
///
/// # Examples
///
/// ```
/// let local_scope = Scope::Local;
/// assert_eq!(local_scope, Scope::Local);
///
/// let threads_scope = Scope::Threads;
/// assert_eq!(threads_scope, Scope::Threads);
/// ```
///
/// # Safety
///
/// This enum does not use any unsafe code.
///
/// # Panics
///
/// This enum does not panic under normal conditions.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scope {
    Local,
    Threads,
}

/// Represents a traced item with various attributes.
///
/// The `Trace` struct contains fields for different attributes of a traced item, such as its name, whether it's validated, whether it enters on poll, its scope, parent, recorder, whether it recurses, whether it's a root, its variables, and whether it's an async trait or function.
///
/// # Examples
///
/// ```
/// let trace = Trace {
///     default: syn::LitBool::new(true, proc_macro2::Span::call_site()),
///     name: syn::LitStr::new("my_trace", proc_macro2::Span::call_site()),
///     validated: syn::LitBool::new(true, proc_macro2::Span::call_site()),
///     enter_on_poll: syn::LitBool::new(false, proc_macro2::Span::call_site()),
///     scope: Some(Scope::Local),
///     // ... and so on for the other fields
/// };
/// assert_eq!(trace.name.value(), "my_trace");
/// ```
///
/// # Safety
///
/// This struct does not use any unsafe code.
///
/// # Panics
///
/// This struct does not panic under normal conditions.
#[derive(Clone, Debug, PartialEq)]
pub struct Trace {
    pub default: syn::LitBool,
    pub name: syn::LitStr,
    pub validated: syn::LitBool,
    pub enter_on_poll: syn::LitBool,

    pub scope: Option<Scope>, // Scope::Local, Scope::Thread, etc.
    pub parent: Option<syn::LitStr>,
    pub recorder: Option<syn::Ident>,
    pub recurse: Option<syn::LitBool>,
    pub root: Option<syn::LitBool>,
    pub variables: Option<syn::ExprArray>,
    pub async_trait: Option<syn::LitBool>,
    pub async_fn: Option<syn::LitBool>,
}

impl syn::parse::Parse for Trace {
    /// Implementation of the `syn::parse::Parse` trait for the `Trace` struct.
    ///
    /// This implementation allows a `Trace` object to be parsed from a `syn::parse::ParseStream`.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `input` is a syn::parse::ParseStream containing valid data for a Trace
    /// let trace = syn::parse::Parse::parse(input).unwrap();
    /// assert_eq!(trace.name.value(), "my_trace");
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The input contains more than 3 arguments.
    /// - The `enter_on_poll` or `name` attributes are provided more than once.
    /// - The value of `enter_on_poll` is not a boolean.
    /// - The value of `name` is not a string.
    /// - An unknown option is provided.
    /// - Both `enter_on_poll` and `name` are missing.
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
    /// `input` - A `syn::parse::ParseStream` from which to parse a `Trace`.
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut enter_on_poll = None;
        let mut name = None;
        let mut name_set = false;

        let mut parsed =
            syn::punctuated::Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated(
                input,
            )?;
        let arg_n = parsed.len();
        if arg_n > 3 {
            // tests/trace/ui/err/has-too-many-arguments.rs
            //abort_call_site!(ERROR; help = HELP)
            let e = syn::Error::new(
                syn::spanned::Spanned::span(&parsed),
                "Too many arguments. This attribute takes up to two (2) arguments",
            );
            return Err(e);
        }
        for kv in parsed.clone() {
            if kv.path.is_ident("enter_on_poll") {
                if enter_on_poll.is_some() {
                    let e = syn::Error::new(
                        syn::spanned::Spanned::span(&kv),
                        "`enter_on_poll` provided twice",
                    );
                    return Err(e);
                } else if let syn::Lit::Bool(v) = kv.lit {
                    enter_on_poll = Some(v);
                } else {
                    let e = syn::Error::new(
                        syn::spanned::Spanned::span(&kv),
                        "`enter_on_poll` value should be an boolean",
                    );
                    return Err(e);
                }
            } else if kv.path.is_ident("name") {
                name_set = true;
                if name.is_some() {
                    let e =
                        syn::Error::new(syn::spanned::Spanned::span(&kv), "`name` provided twice");
                    return Err(e);
                } else if let syn::Lit::Str(v) = kv.lit {
                    name = Some(v);
                } else {
                    let e = syn::Error::new(
                        syn::spanned::Spanned::span(&kv),
                        "`name` value should be a string",
                    );
                    return Err(e);
                }
            } else {
                let e = syn::Error::new(syn::spanned::Spanned::span(&kv), "unknown option");
                return Err(e);
            }
        }

        if !name_set {
            let name_pair: syn::MetaNameValue = syn::parse_quote!(name = "__default");
            parsed.push(name_pair);
            name = Some(syn::LitStr::new(
                "__default",
                proc_macro2::Span::call_site(),
            ));
        }
        // Validate supported combinations
        match (enter_on_poll, name) {
            (Some(enter_on_poll), Some(name)) => {
                let default = syn::LitBool::new(false, proc_macro2::Span::call_site());
                let validated = syn::LitBool::new(true, proc_macro2::Span::call_site());
                Ok(Self {
                    default,
                    enter_on_poll,
                    name,
                    validated,
                    ..Default::default()
                })
            }
            (None, None) => Err(syn::Error::new(
                syn::spanned::Spanned::span(&parsed),
                "missing both `enter_on_poll` and `name`",
            )),
            (None, Some(name)) => {
                let default = syn::LitBool::new(false, proc_macro2::Span::call_site());
                let validated = syn::LitBool::new(true, proc_macro2::Span::call_site());
                Ok(Self {
                    default,
                    name,
                    validated,
                    ..Default::default()
                })
            }
            (Some(enter_on_poll), None) => {
                let default = syn::LitBool::new(false, proc_macro2::Span::call_site());
                let validated = syn::LitBool::new(true, proc_macro2::Span::call_site());
                let name = syn::LitStr::new("__default", proc_macro2::Span::call_site());
                Ok(Self {
                    default,
                    enter_on_poll,
                    name,
                    validated,
                    ..Default::default()
                })
            }
        }
    }
}

/// Implementation of the `std::default::Default` trait for the `Trace` struct.
///
/// This implementation allows a `Trace` object to be created with default values.
///
/// # Examples
///
/// ```
/// let trace = Trace::default();
/// assert_eq!(trace.name.value(), "__default");
/// assert_eq!(trace.default.value(), true);
/// assert_eq!(trace.validated.value(), false);
/// assert_eq!(trace.enter_on_poll.value(), false);
/// assert_eq!(trace.scope.unwrap(), Scope::Local);
/// // ... and so on for the other fields
/// ```
///
/// # Safety
///
/// This function does not use any unsafe code.
///
/// # Panics
///
/// This function does not panic under normal conditions.
impl Default for Trace {
    fn default() -> Self {
        // Indicate when these defaults have changed
        let default = syn::LitBool::new(true, proc_macro2::Span::call_site());
        // Indicate when these values have been validated
        let validated = syn::LitBool::new(false, proc_macro2::Span::call_site());
        let name = syn::LitStr::new("__default", proc_macro2::Span::call_site());
        let scope = Some(Scope::Local);
        let enter_on_poll = syn::LitBool::new(false, proc_macro2::Span::call_site());
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
        let async_fn = Some(syn::LitBool::new(false, proc_macro2::Span::call_site()));

        Self {
            name,
            async_trait,
            async_fn,
            default,
            enter_on_poll,
            parent,
            recorder,
            recurse,
            root,
            scope,
            variables,
            validated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_utilities::*;

    #[test]
    fn valid_trace_001() {
        // let ts = syn::parse::Parser::parse_str(syn::Attribute::parse_outer, "#[trace]").unwrap();
        // let args: proc_macro2::TokenStream = ts
        //     .iter()
        //     .map(|attr| attr.parse_args::<syn::NestedMeta>().unwrap())
        //     .collect();
        let args = quote::quote!(name = "a", enter_on_poll = false,);
        let actual = syn::parse2::<Trace>(args).unwrap();
        let expected = Trace {
            default: syn::LitBool::new(false, proc_macro2::Span::call_site()),
            enter_on_poll: syn::LitBool::new(false, proc_macro2::Span::call_site()),
            name: syn::LitStr::new("a", proc_macro2::Span::call_site()),
            validated: syn::LitBool::new(true, proc_macro2::Span::call_site()),
            ..Default::default()
        };
        assert_eq!(expected, actual);
    }

    #[test]
    fn valid_trace_002() {
        let args = quote::quote!(name = "a", enter_on_poll = false,);
        let actual = syn::parse2::<Trace>(args).unwrap();
        let expected = Trace {
            default: syn::LitBool::new(false, proc_macro2::Span::call_site()),
            enter_on_poll: syn::LitBool::new(false, proc_macro2::Span::call_site()),
            name: syn::LitStr::new("a", proc_macro2::Span::call_site()),
            validated: syn::LitBool::new(true, proc_macro2::Span::call_site()),
            ..Default::default()
        };
        assert_eq!(expected, actual);
    }

    #[test]
    fn valid_trace_003() {
        let args = quote::quote!(enter_on_poll = false,);
        let actual = syn::parse2::<Trace>(args).unwrap();
        let expected = Trace {
            default: syn::LitBool::new(false, proc_macro2::Span::call_site()),
            enter_on_poll: syn::LitBool::new(false, proc_macro2::Span::call_site()),
            name: syn::LitStr::new("__default", proc_macro2::Span::call_site()),
            validated: syn::LitBool::new(true, proc_macro2::Span::call_site()),
            ..Default::default()
        };
        assert_eq!(expected, actual);
    }

    #[test]
    fn valid_trace_004() {
        let args = quote::quote!(name = "a",);
        let actual = syn::parse2::<Trace>(args).unwrap();
        let expected = Trace {
            default: syn::LitBool::new(false, proc_macro2::Span::call_site()),
            name: syn::LitStr::new("a", proc_macro2::Span::call_site()),
            validated: syn::LitBool::new(true, proc_macro2::Span::call_site()),
            ..Default::default()
        };
        assert_eq!(expected, actual);
    }

    #[test]
    fn invalid_trace_001() {
        let args = quote::quote!(name = "a", name = "b", enter_on_poll = false,);
        let actual = match syn::parse2::<Trace>(args.clone()) {
            Err(error) => error,
            _ => syn::Error::new(syn::spanned::Spanned::span(""), "error"),
        };
        let expected: syn::Error =
            syn::Error::new(syn::spanned::Spanned::span(&args), "`name` provided twice");
        assert_eq_text!(&format!("{:#?}", expected), &format!("{:#?}", actual));
    }

    #[test]
    fn invalid_trace_002() {
        let args = quote::quote!(name = "a", enter_on_poll = true, enter_on_poll = false,);
        let actual = match syn::parse2::<Trace>(args.clone()) {
            Err(error) => error,
            _ => syn::Error::new(syn::spanned::Spanned::span(""), "error"),
        };
        let expected: syn::Error = syn::Error::new(
            syn::spanned::Spanned::span(&args),
            "`enter_on_poll` provided twice",
        );
        assert_eq_text!(&format!("{:#?}", expected), &format!("{:#?}", actual));
    }
}
