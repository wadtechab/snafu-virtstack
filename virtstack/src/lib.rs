//! # SNAFU Virtual Stack Trace
//!
//! A lightweight, efficient error handling library for Rust that implements virtual stack traces
//! based on [GreptimeDB's error handling approach](https://greptime.com/blogs/2024-05-07-error-rust).
//! This library combines the power of [SNAFU](https://github.com/shepmaster/snafu) error handling
//! with virtual stack traces to provide meaningful error context without the overhead of system backtraces.
//!
//! ## Motivation
//!
//! Traditional error handling in Rust often faces a dilemma:
//! - **Option 1:** Use system backtraces - long hard to read stack traces only referencing functions and lines
//! - **Option 2:** Simple error propagation - lacks context about where errors originated
//!
//! Virtual stack traces provide a third way: capturing meaningful context at each error propagation point with minimal overhead.
//!
//! ## Features
//!
//! - 🚀 **Lightweight**: Only ~100KB binary overhead vs several MB for system backtraces
//! - 📍 **Precise Location Tracking**: Automatically captures file, line, and column information
//! - 🔗 **Error Chain Walking**: Traverses the entire error source chain
//! - 🎯 **Zero-Cost Abstraction**: Context generation can be postponed until needed
//! - 🛠️ **Seamless Integration**: Works perfectly with SNAFU error handling
//! - 📝 **Developer-Friendly**: Automatic Debug implementation with formatted stack traces
//!
//! ## Basic Usage
//!
//! Simply add the `#[stack_trace_debug]` attribute to your SNAFU error enum:
//!
//! ```rust
//! use snafu::prelude::*;
//! use snafu_virtstack::stack_trace_debug;
//!
//! #[derive(Snafu)]
//! #[stack_trace_debug]  // Add this attribute
//! enum MyError {
//!     #[snafu(display("Failed to read file: {filename}"))]
//!     FileRead { filename: String, source: std::io::Error },
//!     
//!     #[snafu(display("Invalid data format"))]
//!     InvalidFormat { source: serde_json::Error },
//! }
//!
//! fn process_file(filename: &str) -> Result<String, MyError> {
//!     let content = std::fs::read_to_string(filename)
//!         .context(FileReadSnafu { filename })?;
//!     
//!     let data: serde_json::Value = serde_json::from_str(&content)
//!         .context(InvalidFormatSnafu)?;
//!     
//!     Ok(data.to_string())
//! }
//! ```
//!
//! ## Generated Debug Output
//!
//! When an error occurs, the generated [`Debug`] implementation will display:
//!
//! ```text
//! Error: Failed to read file: config.json
//! Virtual Stack Trace:
//!   0: Failed to read file: config.json at src/main.rs:15:23
//!   1: No such file or directory (os error 2) at src/main.rs:16:10
//! ```
//!
//! ## Advanced Usage
//!
//! You can also access the virtual stack programmatically:
//!
//! ```rust
//! use snafu_virtstack::VirtualStackTrace;
//! # use snafu::prelude::*;
//! # use snafu_virtstack::stack_trace_debug;
//! # #[derive(Snafu)]
//! # #[stack_trace_debug]
//! # enum MyError {
//! #     #[snafu(display("Something went wrong"))]
//! #     SomethingWrong,
//! # }
//!
//! let error = MyError::SomethingWrong;
//! let stack = error.virtual_stack();
//!
//! for (i, frame) in stack.iter().enumerate() {
//!     println!("Frame {}: {} at {}:{}",
//!         i,
//!         frame.message,
//!         frame.location.file(),
//!         frame.location.line()
//!     );
//! }
//! ```
//!
//! ## Requirements
//!
//! - Must be applied to `enum` types only
//! - The enum should derive [`Snafu`] for full functionality
//! - Works best with error enums that have source fields for error chaining
//!
//! ## Performance Benefits
//!
//! The virtual stack trace approach provides several key advantages:
//!
//! ### 1. Performance Efficiency
//! Unlike system backtraces that capture the entire call stack (expensive operation),
//! virtual stack traces only record error propagation points. This results in:
//! - Lower CPU usage during error handling
//! - Reduced memory footprint  
//! - Smaller binary sizes (100KB vs several MB)
//!
//! ### 2. Meaningful Context
//! Virtual stack traces capture:
//! - The exact location where each error was propagated
//! - Custom error messages at each level
//! - The complete error chain from root cause to final error
//!
//! ### 3. Production-Ready
//! - Safe to use in production environments
//! - No performance penalties in the happy path
//! - Can be enabled/disabled at runtime if needed
//!
//! ## How It Works
//!
//! 1. **Proc Macro Magic**: The [`stack_trace_debug`] attribute automatically implements:
//!    - [`VirtualStackTrace`] trait for stack frame collection
//!    - Custom [`Debug`] implementation for formatted output
//!
//! 2. **Location Tracking**: Uses Rust's `#[track_caller]` to capture precise locations
//!    where errors are propagated
//!
//! 3. **Error Chain Walking**: Automatically traverses the `source()` chain to build
//!    complete error context
//!
//! 4. **Zero-Cost Until Needed**: Stack frames are only generated when the error is
//!    actually inspected

// Re-export the proc macro so users only need to depend on this crate
pub use snafu_virtstack_macro::stack_trace_debug;

/// Core trait for virtual stack trace functionality.
///
/// This trait is automatically implemented by the [`stack_trace_debug`] proc macro attribute.
/// It provides access to the virtual stack trace showing the error propagation path.
///
/// # Example
///
/// ```rust
/// use snafu::prelude::*;
/// use snafu_virtstack::{stack_trace_debug, VirtualStackTrace};
///
/// #[derive(Snafu)]
/// #[stack_trace_debug]
/// enum MyError {
///     #[snafu(display("Something went wrong"))]
///     SomethingWrong,
/// }
///
/// let error = MyError::SomethingWrong;
/// let stack = error.virtual_stack();
/// for frame in stack {
///     println!("{}", frame);
/// }
/// ```
pub trait VirtualStackTrace {
    /// Returns a virtual stack trace showing error propagation path.
    ///
    /// Each [`StackFrame`] in the returned vector represents one step in the error
    /// propagation chain, from the outermost error context down to the root cause.
    fn virtual_stack(&self) -> Vec<StackFrame>;
}

/// Represents a single frame in the virtual stack trace.
///
/// Each frame captures the location where an error was propagated and the
/// associated error message. This provides precise context about the error
/// propagation path without the overhead of system backtraces.
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Location where the error occurred or was propagated
    pub location: &'static std::panic::Location<'static>,
    /// Error message for this frame
    pub message: String,
}

impl StackFrame {
    /// Creates a new stack frame with the given location and message.
    ///
    /// # Arguments
    ///
    /// * `location` - The location where the error occurred, typically from `std::panic::Location::caller()`
    /// * `message` - A descriptive message for this error frame
    ///
    /// # Example
    ///
    /// ```rust
    /// use snafu_virtstack::StackFrame;
    /// use std::panic::Location;
    ///
    /// #[track_caller]
    /// fn create_frame() -> StackFrame {
    ///     StackFrame::new(
    ///         Location::caller(),
    ///         "Something went wrong".to_string()
    ///     )
    /// }
    /// ```
    pub fn new(location: &'static std::panic::Location<'static>, message: String) -> Self {
        Self { location, message }
    }
}

impl std::fmt::Display for StackFrame {
    /// Formats the stack frame showing the message and location information.
    ///
    /// The format is: `{message} at {file}:{line}:{column}`
    ///
    /// # Example Output
    ///
    /// ```text
    /// Failed to read configuration file at src/config.rs:42:15
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} at {}:{}:{}",
            self.message,
            self.location.file(),
            self.location.line(),
            self.location.column()
        )
    }
}
