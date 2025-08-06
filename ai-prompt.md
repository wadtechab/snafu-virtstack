# Error Handling with Snafu

This guide covers how to implement robust error handling using [Snafu](https://crates.io/crates/snafu) with `snafu-virtstack` crate.

## Quick Start

```rust
use snafu_virtstack::stack_trace_debug;
use snafu::prelude::*;

#[derive(Snafu)]
#[stack_trace_debug]
enum AppError {
    #[snafu(display("Failed to query database: {source}"))]
    FailedQueringDatabase { source: std::io::Error },

    #[snafu(display("Invalid input: {input}"))]
    InvalidInput { input: String },
}

fn database_operation() -> Result<(), AppError> {
    create_io_error().context(DatabaseSnafu)
}
```

## Core Components

### `snafu_virtstack`
- **`VirtualStackTrace`** trait: Provides virtual stack trace functionality
- **`StackFrame`** struct: Represents single frames in error propagation
- **`#[stack_trace_debug]`** attribute: Auto-generates:
  - `VirtualStackTrace` implementation
  - Custom `Debug` formatter showing virtual stack traces

## Error Definition Patterns

### Basic Error Enum
```rust
#[derive(Snafu)]
#[stack_trace_debug]
enum MyError {
    #[snafu(display("Operation failed"))]
    OperationFailed,

    #[snafu(display("File not found: {path}"))]
    FileNotFound { path: String },
}
```

### With Source Errors
```rust
#[derive(Snafu)]
#[stack_trace_debug]
enum MyError {
    #[snafu(display("IO operation failed: {source}"))]
    Io { source: std::io::Error },

    #[snafu(display("Failed to parse input `{input}`: {source}"))]
    Parse { input: String, source: serde_json::Error },

    #[snafu(display("Filesystem IO issue: {source}"))]
    FilesystemIoFailure { source: Box<dyn std::error::Error + Send + Sync> },
}
```

### Error Chains
```rust
#[derive(Snafu)]
#[stack_trace_debug]
enum ServiceError {
    #[snafu(display("Service unavailable: {source}"))]
    ServiceUnavailable { source: AppError },
}

fn service_operation() -> Result<(), ServiceError> {
    database_operation().context(ServiceUnavailableSnafu)
}
```

## Context Usage

### Apply Context with `.context()`
```rust
fn read_config() -> Result<String, MyError> {
    std::fs::read_to_string("config.toml")
        .context(IoSnafu)?;
    Ok("config".to_string())
}
```

### Boxing errors
```rust
fn read_config() -> Result<String, MyError> {
    std::fs::read_to_string("config.toml")
        .boxed()
        .context(FilesystemIoFailureSnafu)?;
    Ok("config".to_string())
}
```

### Build Errors Directly
```rust
fn validate_input(input: &str) -> Result<(), MyError> {
    if input.is_empty() {
        return FileNotFoundSnafu {
            path: "input.txt".to_string()
        }.fail();
    }
    Ok(())
}
```

### Using `ensure!` Macro
```rust
fn check_range(value: usize) -> Result<(), MyError> {
    ensure!(value < 100, OperationFailedSnafu);
    Ok(())
}
```

## Advanced Features

### Custom Context Names
```rust
#[derive(Snafu)]
#[stack_trace_debug]
enum Error {
    #[snafu(context(name(MyCustomContext)))]
    SomeError,
}
```

### Snafu generation scope

If we have several error types with colliding enum variants, then the <VariantSnafu> will collide.
To avoid this, we can scope errors and then use `my_error::WrappedSnafu`.

```rust
mod my_error {
    use snafu::prelude::*;
    use snafu_virtstack::stack_trace_debug;

    #[derive(Snafu)]
    #[snafu(visibility(pub))]
    #[stack_trace_debug]
    enum MyError {
        #[snafu(display("Inner failure: {source}"))]
        Wrapped { source: SomeOtherError },
    }
}
use my_error::MyError;
```

## Best Practices

1. **Always use `#[stack_trace_debug]`** for enhanced debugging
2. **Provide meaningful display messages** with context
3. **Chain errors appropriately** to maintain error context
4. **Use context selectors** (e.g., `FailedQueringDatabaseSnafu`) instead of enum variants directly
5. **Leverage `ensure!`** for conditional error creation
6. **Name variants** by the context we'll use them for, like `GetAccount` and not `AccountManagerError`.

## Output Example

With `#[stack_trace_debug]`, errors display virtual stack traces:

```
Error: Service unavailable
Virtual Stack Trace:
  0: Service unavailable at src/main.rs:35:36
  1: Database connection failed at src/main.rs:31:32
  2: File not found at src/main.rs:25:5
```

This provides clear error propagation paths without performance overhead of full backtraces.
