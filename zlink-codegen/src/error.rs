use derive_more::From;

/// Error types that can occur during Varlink interface code generation.
#[derive(Debug, From)]
pub enum Error {
    /// An invalid argument was provided
    InvalidArgument,

    /// The code generation process failed.
    CodegenFailed,

    /// The generated code formatting failed.
    FormatFailed,

    /// An I/O error occurred during file operations.
    #[from]
    Io(std::io::Error),

    /// An error from the zlink-core library.
    #[from]
    Zlink(zlink::Error)
}

// Nicely print errors together with #[from] from derive_more
impl core::fmt::Display for Error {
   fn fmt(
       &self,
       fmt: &mut core::fmt::Formatter<'_>,
   ) -> core::result::Result<(), core::fmt::Error> {
       write!(fmt, "{self:?}")
   }
}

impl std::error::Error for Error {}
