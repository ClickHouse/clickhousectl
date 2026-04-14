use serde::{Deserialize, Serialize};

/// Strict string enum — rejects unknown values during deserialization.
/// Use for enums that represent user input or CLI-only values.
///
/// Callers must have `use std::fmt;`, `use std::ops::Deref;`, and
/// `use std::str::FromStr;` in scope at the expansion site.
#[allow(unused_macros)]
macro_rules! string_enum {
    ($(#[$meta:meta])* pub enum $name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $(
                #[serde(rename = $value)]
                $variant,
            )+
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let value = match self {
                    $(Self::$variant => $value,)+
                };
                f.write_str(value)
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                match self {
                    $(Self::$variant => $value,)+
                }
            }
        }

        impl FromStr for $name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(format!(
                        "unknown value '{}', expected one of: {}",
                        s,
                        [$($value),+].join(", ")
                    )),
                }
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.deref() == *other
            }
        }

        impl PartialEq<$name> for &str {
            fn eq(&self, other: &$name) -> bool {
                *self == other.deref()
            }
        }
    };
}

/// Flexible string enum — accepts unknown values from API responses.
/// Use for enums that appear in API response types where the server may
/// return new values the CLI doesn't know about yet.
///
/// Callers must have `use std::fmt;`, `use std::ops::Deref;`, and
/// `use std::str::FromStr;` in scope at the expansion site.
#[allow(unused_macros)]
macro_rules! flexible_string_enum {
    ($(#[$meta:meta])* pub enum $name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum $name {
            $($variant,)+
            /// Unknown value returned by the API that this CLI version doesn't recognize.
            Other(String),
        }

        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                match self {
                    $(Self::$variant => serializer.serialize_str($value),)+
                    Self::Other(s) => serializer.serialize_str(s),
                }
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let s = String::deserialize(deserializer)?;
                Ok(match s.as_str() {
                    $($value => Self::$variant,)+
                    _ => Self::Other(s),
                })
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $(Self::$variant => f.write_str($value),)+
                    Self::Other(s) => f.write_str(s),
                }
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                match self {
                    $(Self::$variant => $value,)+
                    Self::Other(s) => s.as_str(),
                }
            }
        }

        impl $name {
            /// Returns the list of known string values for this enum.
            pub fn known_values() -> &'static [&'static str] {
                &[$($value),+]
            }
        }

        impl FromStr for $name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(format!(
                        "unknown value '{}', expected one of: {}",
                        s,
                        [$($value),+].join(", ")
                    )),
                }
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.deref() == *other
            }
        }

        impl PartialEq<$name> for &str {
            fn eq(&self, other: &$name) -> bool {
                *self == other.deref()
            }
        }
    };
}

/// Standard API response wrapper
#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub result: Option<T>,
    #[allow(dead_code)]
    pub error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    #[allow(dead_code)]
    pub code: Option<String>,
    pub message: String,
}

/// Delete service success payload returned directly by the API without a result wrapper.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResponse {
    pub status: f64,
    pub request_id: String,
}
