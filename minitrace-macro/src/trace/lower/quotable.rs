/// `Quotables` is a newtype for a vector of generic type `T`.
///
/// This struct serves as a wrapper that allows us to implement the [`From`] trait.
/// The [`From`] trait provides these conveniences (`match` branch):
///
///     Err(err) => return err.into_compile_error().into(),
///
/// # Examples
///
/// ```
/// // Assuming `quotables` is a `Quotables` instance
/// // and `item` is of type `T`
/// quotables.push(item);
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
/// This struct does not use any lifetimes.
///
/// # Notes
///
/// The `Quotables` struct is a newtype for a vector of generic type `T`. It allows us to implement the [`From`] trait, which provides certain conveniences.
/// The following traits are implemented for `Quotables`:
///
/// - Debug (via #[derive(...)])
/// - Default
/// - Deref
/// - DerefMut
/// - Display
#[derive(Debug, Clone)]
pub struct Quotables<T>(Vec<T>);

impl<T: std::fmt::Debug> Quotables<T> {
    /// Creates a new `Quotables` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// let quotables = Quotables::<i32>::new();
    /// ```
    ///
    /// # Safety
    ///
    /// This method does not use any unsafe code.
    ///
    /// # Panics
    ///
    /// This method does not panic under normal conditions.
    ///
    /// # Notes
    ///
    /// The `new` method is used to create a new `Quotables` instance. It initializes the inner vector with an empty vector.
    pub fn new() -> Quotables<T> {
        Quotables(Vec::<T>::new())
    }

    /// Creates a new `Quotables` instance with a specified capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// let quotables = Quotables::<i32>::with_capacity(10);
    /// ```
    ///
    /// # Safety
    ///
    /// This method does not use any unsafe code.
    ///
    /// # Panics
    ///
    /// This method does not panic under normal conditions.
    ///
    /// # Arguments
    ///
    /// `capacity` - The capacity for the new `Quotables` instance.
    ///
    /// # Notes
    ///
    /// The `with_capacity` method is used to create a new `Quotables` instance with a specified capacity. It initializes the inner vector with a vector of the given capacity.
    #[allow(dead_code)]
    pub fn with_capacity(capacity: usize) -> Quotables<T> {
        Quotables(Vec::<T>::with_capacity(capacity))
    }
}

/// Provides a default value for `Quotables`.
///
/// # Examples
///
/// ```
/// let quotables = Quotables::<i32>::default();
/// ```
///
/// # Safety
///
/// This method does not use any unsafe code.
///
/// # Panics
///
/// This method does not panic under normal conditions.
///
/// # Notes
///
/// The `default` method is used to provide a default value for `Quotables`. It simply calls the `new` method of `Quotables`.
impl<T: std::fmt::Debug> Default for Quotables<T> {
    fn default() -> Quotables<T> {
        Quotables::new()
    }
}

/// Formats the `Quotables` for display.
///
/// # Examples
///
/// ```
/// // Assuming `quotables` is a `Quotables` instance
/// println!("{}", quotables);
/// ```
///
/// # Errors
///
/// Returns an error if the underlying formatter returns an error.
///
/// # Safety
///
/// This method does not use any unsafe code.
///
/// # Panics
///
/// This method does not panic under normal conditions.
///
/// # Arguments
///
/// `f` - The output formatter.
///
/// # Notes
///
/// The `fmt` method is used to format the `Quotables` for display. It uses the `Debug` implementation of the inner vector.
impl<T: std::fmt::Debug> std::fmt::Display for Quotables<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
/// Provides a reference to the inner vector of `Quotables`.
///
/// # Examples
///
/// ```
/// // Assuming `quotables` is a `Quotables` instance
/// let vec_ref: &Vec<i32> = &*quotables;
/// ```
///
/// # Safety
///
/// This method does not use any unsafe code.
///
/// # Panics
///
/// This method does not panic under normal conditions.
///
/// # Notes
///
/// The `deref` method is used to provide a reference to the inner vector of `Quotables`.
impl<T: std::fmt::Debug> std::ops::Deref for Quotables<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Vec<T> {
        &self.0
    }
}

/// Provides a mutable reference to the inner vector of `Quotables`.
///
/// # Examples
///
/// ```
/// // Assuming `quotables` is a `Quotables` instance
/// let vec_mut_ref: &mut Vec<i32> = &mut *quotables;
/// ```
///
/// # Safety
///
/// This method does not use any unsafe code.
///
/// # Panics
///
/// This method does not panic under normal conditions.
///
/// # Notes
///
/// The `deref_mut` method is used to provide a mutable reference to the inner vector of `Quotables`.
impl<T: std::fmt::Debug> std::ops::DerefMut for Quotables<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Represents a quotable item.
///
/// This enum can be used to represent a quotable item. It currently has only one variant, `Item`, which holds a `Quote`.
///
/// # Examples
///
/// ```
/// let quote = Quote::new("Hello, world!");
/// let quotable = Quotable::Item(quote);
/// ```
///
/// # Errors
///
/// This enum has an associated error type, which is used to represent validation logic errors.
///
/// # Safety
///
/// This enum does not use any unsafe code.
///
/// # Panics
///
/// This enum does not panic under normal conditions.
///
/// # Notes
///
/// The `Quotable` enum is used to represent a quotable item. It currently has only one variant, `Item`, which holds a `Quote`.
/// The enum is marked with `#[allow(dead_code)]` to suppress warnings about the enum not being used in the code.
/// The `Clone`, `Debug`, and `thiserror::Error` traits are derived for this enum.
#[allow(dead_code)]
#[derive(Clone, Debug, thiserror::Error)]
#[error("Validation logic error")]
pub enum Quotable {
    Item(Quote),
}

/// Represents a quotable function.
///
/// This struct can be used to represent a quotable function. It holds all the necessary information to generate a function.
///
/// # Examples
///
/// ```
/// // Assuming `attrs`, `vis`, `constness`, `unsafety`, `abi`, `ident`, `gen_params`, `params`, `return_type`, `where_clause`, and `func_body` are properly initialized
/// let quote = Quote {
///     attrs,
///     vis,
///     constness,
///     unsafety,
///     abi,
///     ident,
///     gen_params,
///     params,
///     return_type,
///     where_clause,
///     func_body,
/// };
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
/// # Notes
///
/// The `Quote` struct is used to represent a quotable function. It holds all the necessary information to generate a function.
/// The `Clone`, `Debug`, and `thiserror::Error` traits are derived for this struct.
#[derive(Clone, Debug, thiserror::Error)]
pub struct Quote {
    pub attrs: Vec<syn::Attribute>,
    pub vis: syn::Visibility,
    pub constness: Option<syn::token::Const>,
    pub unsafety: Option<syn::token::Unsafe>,
    pub abi: Option<syn::Abi>,
    pub ident: syn::Ident,
    pub gen_params: syn::punctuated::Punctuated<syn::GenericParam, syn::Token![,]>,
    pub params: syn::punctuated::Punctuated<syn::FnArg, syn::Token![,]>,
    pub return_type: syn::ReturnType,
    pub where_clause: Option<syn::WhereClause>,
    pub func_body: proc_macro2::TokenStream,
}

/// Formats the `Quote` for display.
///
/// # Examples
///
/// ```
/// // Assuming `quote` is a `Quote` instance
/// println!("{}", quote);
/// ```
///
/// # Errors
///
/// Returns an error if the underlying formatter returns an error.
///
/// # Safety
///
/// This method does not use any unsafe code.
///
/// # Panics
///
/// This method does not panic under normal conditions.
///
/// # Arguments
///
/// `f` - The output formatter.
///
/// # Notes
///
/// The `fmt` method is used to format the `Quote` for display. It uses the `Debug` implementation of the `Quote`.
impl std::fmt::Display for Quote {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
