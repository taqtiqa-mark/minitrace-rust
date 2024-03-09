/// Collects lifetime information from a given item.
///
/// The `CollectLifetimes` struct is used to collect both elided and explicit lifetimes from a given item.
/// It also contains a name for identification and a default span for error reporting.
///
/// # Examples
///
/// ```
/// // Assuming `default_span` is a proc_macro2::Span for the current span
/// let collector = CollectLifetimes {
///     elided: Vec::new(),
///     explicit: Vec::new(),
///     name: "my_collector",
///     default_span: default_span,
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
/// # Lifetimes
///
/// This struct does not use any lifetimes.
///
/// # Notes
///
/// The `CollectLifetimes` struct is used to collect lifetime information from a given item. It is used in the process
/// of parsing and analyzing Rust code. The `elided` and `explicit` fields are used to store the collected lifetimes,
/// while the `name` field is used for identification purposes. The `default_span` field is used for error reporting.
pub struct CollectLifetimes {
    pub elided: Vec<syn::Lifetime>,
    pub explicit: Vec<syn::Lifetime>,
    pub name: &'static str,
    pub default_span: proc_macro2::Span,
}

impl CollectLifetimes {
    /// Creates a new `CollectLifetimes` instance.
    ///
    /// This method initializes a new `CollectLifetimes` instance with the provided name and default span.
    /// The `elided` and `explicit` fields are initialized as empty vectors.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `default_span` is a proc_macro2::Span for the current span
    /// let collector = CollectLifetimes::new("my_collector", default_span);
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
    /// `name` - The name for the `CollectLifetimes` instance.
    ///
    /// `default_span` - The default span for the `CollectLifetimes` instance.
    ///
    /// # Notes
    ///
    /// The `new` method is used to create a new `CollectLifetimes` instance. It takes a name and a default span as arguments,
    /// and initializes the `elided` and `explicit` fields as empty vectors.
    pub fn new(name: &'static str, default_span: proc_macro2::Span) -> Self {
        CollectLifetimes {
            elided: Vec::new(),
            explicit: Vec::new(),
            name,
            default_span,
        }
    }

    /// Visits an optional lifetime.
    ///
    /// This method checks if the lifetime is `None` or `Some`. If it's `None`, it will be replaced with a new lifetime.
    /// If it's `Some`, the lifetime will be visited using the `visit_lifetime` method.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `collector` is a `CollectLifetimes` instance
    /// // and `lifetime` is an optional `syn::Lifetime`
    /// collector.visit_opt_lifetime(&mut lifetime);
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
    /// # Lifetimes
    ///
    /// This method does not use any lifetimes.
    ///
    /// # Arguments
    ///
    /// `lifetime` - The optional lifetime to visit.
    ///
    /// # Notes
    ///
    /// The `visit_opt_lifetime` method is used to visit an optional lifetime. If the lifetime is `None`, it will be replaced with a new lifetime.
    /// If it's `Some`, the lifetime will be visited using the `visit_lifetime` method.
    pub fn visit_opt_lifetime(&mut self, lifetime: &mut Option<syn::Lifetime>) {
        match lifetime {
            None => *lifetime = Some(self.next_lifetime(None)),
            Some(lifetime) => self.visit_lifetime(lifetime),
        }
    }

    /// Visits a lifetime.
    ///
    /// This method checks if the lifetime is elided (i.e., its identifier is `_`). If it is, it will be replaced with a new lifetime.
    /// Otherwise, it will be added to the list of explicit lifetimes.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `collector` is a `CollectLifetimes` instance
    /// // and `lifetime` is a `syn::Lifetime`
    /// collector.visit_lifetime(&mut lifetime);
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
    /// # Lifetimes
    ///
    /// This method does not use any lifetimes.
    ///
    /// # Arguments
    ///
    /// `lifetime` - The lifetime to visit.
    ///
    /// # Notes
    ///
    /// The `visit_lifetime` method is used to visit a lifetime. If the lifetime is elided (i.e., its identifier is `_`), it will be replaced with a new lifetime.
    /// Otherwise, it will be added to the list of explicit lifetimes.
    pub fn visit_lifetime(&mut self, lifetime: &mut syn::Lifetime) {
        if lifetime.ident == "_" {
            *lifetime = self.next_lifetime(lifetime.span());
        } else {
            self.explicit.push(lifetime.clone());
        }
    }

    /// Generates a new lifetime.
    ///
    /// This method creates a new lifetime with a unique name and a given span. The new lifetime is also added to the list of elided lifetimes.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `collector` is a `CollectLifetimes` instance
    /// // and `span` is a `proc_macro2::Span`
    /// let new_lifetime = collector.next_lifetime(span);
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
    /// # Lifetimes
    ///
    /// This method does not use any lifetimes.
    ///
    /// # Arguments
    ///
    /// `span` - The optional span for the new lifetime. If `None` is provided, the default span of the `CollectLifetimes` instance will be used.
    ///
    /// # Notes
    ///
    /// The `next_lifetime` method is used to generate a new lifetime. The new lifetime's name will be the name of the `CollectLifetimes` instance followed by the number of elided lifetimes.
    /// The span of the new lifetime will be the given span, or the default span if no span is given. The new lifetime is also added to the list of elided lifetimes.
    pub fn next_lifetime<S: Into<Option<proc_macro2::Span>>>(&mut self, span: S) -> syn::Lifetime {
        let name = format!("{}{}", self.name, self.elided.len());
        let span = span.into().unwrap_or(self.default_span);
        let life = syn::Lifetime::new(&name, span);
        self.elided.push(life.clone());
        life
    }
}

impl syn::visit_mut::VisitMut for CollectLifetimes {
    /// Visits a mutable receiver.
    ///
    /// This method checks if the receiver has a reference. If it does, the method visits the optional lifetime of the reference.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `collector` is a `CollectLifetimes` instance
    /// // and `receiver` is a `syn::Receiver`
    /// collector.visit_receiver_mut(&mut receiver);
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
    /// # Lifetimes
    ///
    /// This method does not use any lifetimes.
    ///
    /// # Arguments
    ///
    /// `arg` - The mutable receiver to visit.
    ///
    /// # Notes
    ///
    /// The `visit_receiver_mut` method is used to visit a mutable receiver. If the receiver has a reference, the method visits the optional lifetime of the reference.
    pub fn visit_receiver_mut(&mut self, arg: &mut syn::Receiver) {
        if let Some((_, lifetime)) = &mut arg.reference {
            self.visit_opt_lifetime(lifetime);
        }
    }

    /// Visits a mutable type reference.
    ///
    /// This method visits the optional lifetime of the type reference, and then visits the type reference itself.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `collector` is a `CollectLifetimes` instance
    /// // and `type_ref` is a `syn::TypeReference`
    /// collector.visit_type_reference_mut(&mut type_ref);
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
    /// # Lifetimes
    ///
    /// This method does not use any lifetimes.
    ///
    /// # Arguments
    ///
    /// `ty` - The mutable type reference to visit.
    ///
    /// # Notes
    ///
    /// The `visit_type_reference_mut` method is used to visit a mutable type reference. It first visits the optional lifetime of the type reference,
    /// and then visits the type reference itself using the `visit_type_reference_mut` method from the `syn::visit_mut` module.
    pub fn visit_type_reference_mut(&mut self, ty: &mut syn::TypeReference) {
        self.visit_opt_lifetime(&mut ty.lifetime);
        syn::visit_mut::visit_type_reference_mut(self, ty);
    }

    /// Visits a mutable generic argument.
    ///
    /// This method checks if the generic argument is a lifetime. If it is, the method visits the lifetime.
    /// After that, it visits the generic argument itself.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assuming `collector` is a `CollectLifetimes` instance
    /// // and `gen_arg` is a `syn::GenericArgument`
    /// collector.visit_generic_argument_mut(&mut gen_arg);
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
    /// # Lifetimes
    ///
    /// This method does not use any lifetimes.
    ///
    /// # Arguments
    ///
    /// `gen` - The mutable generic argument to visit.
    ///
    /// # Notes
    ///
    /// The `visit_generic_argument_mut` method is used to visit a mutable generic argument. If the generic argument is a lifetime, the method visits the lifetime.
    /// After that, it visits the generic argument itself using the `visit_generic_argument_mut` method from the `syn::visit_mut` module.
    pub fn visit_generic_argument_mut(&mut self, gen: &mut syn::GenericArgument) {
        if let syn::GenericArgument::Lifetime(lifetime) = gen {
            self.visit_lifetime(lifetime);
        }
        syn::visit_mut::visit_generic_argument_mut(self, gen);
    }
}
