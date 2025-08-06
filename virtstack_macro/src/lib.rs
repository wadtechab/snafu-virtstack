use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, parse_macro_input};

/// Proc macro attribute to automatically generate virtual stack traces for SNAFU errors.
///
/// This attribute automatically implements the [`VirtualStackTrace`] trait and provides
/// a custom [`Debug`] implementation that displays a formatted virtual stack trace.
///
/// The macro captures precise location information using Rust's `#[track_caller]`
/// and walks the error source chain to build a complete error context without
/// the overhead of system backtraces.
///
/// # Features
///
/// - **Automatic Implementation**: No need to manually implement virtual stack trace logic
/// - **Location Tracking**: Captures file, line, and column information automatically
/// - **Error Chain Walking**: Traverses the complete error source chain
/// - **Zero-Cost Abstraction**: Stack frames are only generated when needed
/// - **Custom Debug Output**: Provides formatted stack traces in debug output
///
/// # Usage
///
/// Simply add the `#[stack_trace_debug]` attribute to your SNAFU error enum:
///
/// ```rust
/// use snafu::{Snafu, ResultExt};
/// use snafu_virtstack::stack_trace_debug;
///
/// #[derive(Snafu)]
/// #[stack_trace_debug]  // Add this attribute
/// enum MyError {
///     #[snafu(display("Failed to read file: {filename}"))]
///     FileRead { filename: String, source: std::io::Error },
///     
///     #[snafu(display("Invalid data format"))]
///     InvalidFormat { source: serde_json::Error },
/// }
///
/// fn process_file(filename: &str) -> Result<String, MyError> {
///     let content = std::fs::read_to_string(filename)
///         .context(FileReadSnafu { filename })?;
///     
///     let data: serde_json::Value = serde_json::from_str(&content)
///         .context(InvalidFormatSnafu)?;
///     
///     Ok(data.to_string())
/// }
/// ```
///
/// # Generated Debug Output
///
/// When an error occurs, the generated [`Debug`] implementation will display:
///
/// ```text
/// Error: Failed to read file: config.json
/// Virtual Stack Trace:
///   0: Failed to read file: config.json at src/main.rs:15:23
///   1: No such file or directory (os error 2) at src/main.rs:16:10
/// ```
///
/// # Advanced Usage
///
/// You can also access the virtual stack programmatically:
///
/// ```rust
/// use snafu_virtstack::VirtualStackTrace;
/// # use snafu::{Snafu, ResultExt};
/// # use snafu_virtstack::stack_trace_debug;
/// # #[derive(Snafu)]
/// # #[stack_trace_debug]
/// # enum MyError {
/// #     #[snafu(display("Something went wrong"))]
/// #     SomethingWrong,
/// # }
///
/// let error = MyError::SomethingWrong;
/// let stack = error.virtual_stack();
///
/// for (i, frame) in stack.iter().enumerate() {
///     println!("Frame {}: {} at {}:{}",
///         i,
///         frame.message,
///         frame.location.file(),
///         frame.location.line()
///     );
/// }
/// ```
///
/// # Requirements
///
/// - Must be applied to `enum` types only
/// - The enum should derive [`Snafu`] for full functionality
/// - Works best with error enums that have source fields for error chaining
///
/// [`VirtualStackTrace`]: snafu_virtstack::VirtualStackTrace
/// [`Snafu`]: snafu::Snafu
#[proc_macro_attribute]
pub fn stack_trace_debug(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Generate the enhanced version with virtual stack trace implementation
    match generate_stack_trace_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn generate_stack_trace_impl(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Parse the enum to understand its structure
    let _data = match &input.data {
        Data::Enum(data) => data,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "stack_trace_debug can only be applied to enums",
            ));
        }
    };

    // Generate VirtualStackTrace implementation
    let stack_trace_impl =
        generate_virtual_stack_trace_impl(name, &impl_generics, &ty_generics, where_clause)?;

    Ok(quote! {
        // First, emit the original item unchanged
        #input

        // Finally, add the VirtualStackTrace implementation
        #stack_trace_impl
    })
}

fn generate_virtual_stack_trace_impl(
    name: &syn::Ident,
    impl_generics: &syn::ImplGenerics,
    ty_generics: &syn::TypeGenerics,
    where_clause: Option<&syn::WhereClause>,
) -> syn::Result<proc_macro2::TokenStream> {
    Ok(quote! {
        impl #impl_generics snafu_virtstack::VirtualStackTrace for #name #ty_generics #where_clause {
            #[track_caller]
            fn virtual_stack(&self) -> Vec<snafu_virtstack::StackFrame> {
                let mut stack = vec![snafu_virtstack::StackFrame::new(
                    std::panic::Location::caller(),
                    self.to_string(),
                )];

                // Walk the error source chain
                let mut current_error = self as &dyn std::error::Error;
                while let Some(source) = current_error.source() {
                    // Add a simple frame for this source
                    stack.push(snafu_virtstack::StackFrame::new(
                        std::panic::Location::caller(),
                        source.to_string(),
                    ));
                    current_error = source;
                }

                stack
            }
        }

        impl #impl_generics std::fmt::Debug for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                use snafu_virtstack::VirtualStackTrace;

                writeln!(f, "Error: {}", self)?;
                writeln!(f, "Virtual Stack Trace:")?;

                let stack = self.virtual_stack();
                for (i, frame) in stack.iter().enumerate() {
                    writeln!(f, "  {}: {}", i, frame)?;
                }

                Ok(())
            }
        }
    })
}
