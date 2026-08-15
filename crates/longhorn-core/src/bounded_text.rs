//! The [`bounded_text!`](crate::bounded_text) macro.

/// Generates a nonempty, hard-bounded text newtype and its validation error.
///
/// One invocation produces a `String` newtype whose `new` rejects empty input
/// and input beyond the named ceiling, plus a matching `Empty`/`TooLong`
/// error enum with hand-written messages. Domains share the shape so a label
/// edited in one does not validate differently in another.
#[macro_export]
macro_rules! bounded_text {
    ($name:ident, $error:ident, $maximum:ident, $noun:literal) => {
        #[doc = concat!("Nonempty bounded ", $noun, ".")]
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name(::std::string::String);

        impl $name {
            #[doc = concat!("Validates and constructs a ", $noun, ".")]
            pub fn new(
                value: impl Into<::std::string::String>,
            ) -> ::std::result::Result<Self, $error> {
                let value = value.into();
                if value.is_empty() {
                    return ::std::result::Result::Err($error::Empty);
                }
                if value.len() > $maximum {
                    return ::std::result::Result::Err($error::TooLong {
                        maximum: $maximum,
                        actual: value.len(),
                    });
                }
                ::std::result::Result::Ok(Self(value))
            }

            #[doc = concat!("Returns the ", $noun, ".")]
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        #[doc = concat!("Invalid ", $noun, ".")]
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum $error {
            /// The text was empty.
            Empty,
            /// The text exceeded its hard byte ceiling.
            TooLong {
                /// Hard byte ceiling.
                maximum: usize,
                /// Supplied byte count.
                actual: usize,
            },
        }

        impl ::core::fmt::Display for $error {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    Self::Empty => {
                        write!(formatter, concat!($noun, " cannot be empty"))
                    }
                    Self::TooLong { maximum, actual } => write!(
                        formatter,
                        concat!($noun, " is {} bytes; maximum is {}"),
                        actual, maximum
                    ),
                }
            }
        }

        impl ::std::error::Error for $error {}
    };
}
