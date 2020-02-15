//! # onedrive-api
//!
//! `onedrive-api` crate provides middle-level HTTP APIs [`OneDrive`][one_drive] to the
//! [OneDrive][ms_onedrive] API through [Microsoft Graph][ms_graph], and also [`Authentication`][auth]
//! with utilities for authentication.
//!
//! Async support is TODO.
//!
//! ## Example
//! ```ignore
//! use onedrive_api::{OneDrive, FileName, DriveLocation, ItemLocation};
//! use reqwest::Client;
//!
//! # fn run() -> onedrive_api::Result<()> {
//! let client = Client::new();
//! let drive = OneDrive::new(
//!     "<...TOKEN...>".to_owned(), // Login token to Microsoft Graph.
//!     DriveLocation::me(),
//! );
//!
//! let folder_item = drive
//!     .create_folder(
//!         ItemLocation::root(),
//!         FileName::new("test_folder").unwrap(),
//!     )?;
//!
//! drive
//!     .upload_small(
//!         folder_item.id.as_ref().unwrap(),
//!         b"Hello, world",
//!     )?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! [ms_onedrive]: https://onedrive.live.com/about
//! [ms_graph]: https://docs.microsoft.com/graph/overview
//! [one_drive]: ./struct.OneDrive.html
//! [auth]: ./struct.Authentication.html
//! [api]: ./trait.Api.html
//! [api_execute]: ./trait.Api.html#tymethod.execute
//! [client]: ./trait.Client.html
// #![deny(warnings)]
#![deny(missing_debug_implementations)]
#![deny(missing_docs)]
use serde::{de, Serialize};

mod auth;
mod error;
mod onedrive;
pub mod option;
pub mod resource;
mod util;

pub use self::{
    auth::{Authentication, Permission, Token},
    error::{Error, Result},
    onedrive::{
        CopyProgress, CopyProgressMonitor, CopyStatus, ListChildrenFetcher, OneDrive,
        TrackChangeFetcher, UploadSession,
    },
    resource::{DriveId, ItemId, Tag},
    util::{DriveLocation, FileName, ItemLocation},
};

/// The conflict resolution behavior for actions that create a new item.
///
/// # See also
/// [Microsoft Docs](https://docs.microsoft.com/en-us/graph/api/resources/driveitem?view=graph-rest-1.0#instance-attributes)
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConflictBehavior {
    /// Make the request fail. Usually cause HTTP 409 CONFLICT.
    Fail,
    /// **DANGER**: Replace the existing item.
    Replace,
    /// Rename the newly created item to another name.
    ///
    /// The new name is not specified and usually can be retrived from the response.
    Rename,
}

/// A half-open byte range `start..end` or `start..`.
#[derive(Debug, PartialEq, Eq)]
pub struct ExpectRange {
    /// The lower bound of the range (inclusive).
    pub start: u64,
    /// The optional upper bound of the range (exclusive).
    pub end: Option<u64>,
}

impl<'de> de::Deserialize<'de> for ExpectRange {
    fn deserialize<D: de::Deserializer<'de>>(
        deserializer: D,
    ) -> ::std::result::Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = ExpectRange;

            fn expecting(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                write!(f, "Expect Range")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> ::std::result::Result<Self::Value, E> {
                let parse = || -> Option<ExpectRange> {
                    let mut it = v.split('-');
                    let start = it.next()?.parse().ok()?;
                    let end = match it.next()? {
                        "" => None,
                        s => {
                            let end = s.parse::<u64>().ok()?.checked_add(1)?; // Exclusive.
                            if end <= start {
                                return None;
                            }
                            Some(end)
                        }
                    };
                    if it.next().is_some() {
                        return None;
                    }

                    Some(ExpectRange { start, end })
                };
                match parse() {
                    Some(v) => Ok(v),
                    None => Err(E::invalid_value(
                        de::Unexpected::Str(v),
                        &"`{lower}-` or `{lower}-{upper}`",
                    )),
                }
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_parsing() {
        let cases = [
            (
                "42-196",
                Some(ExpectRange {
                    start: 42,
                    end: Some(197),
                }),
            ), // [left, right)
            (
                "418-",
                Some(ExpectRange {
                    start: 418,
                    end: None,
                }),
            ),
            ("", None),
            ("42-4", None),
            ("-9", None),
            ("-", None),
            ("1-2-3", None),
            ("0--2", None),
            ("-1-2", None),
        ];

        for &(s, ref expect) in &cases {
            let ret = serde_json::from_str(&serde_json::to_string(s).unwrap());
            assert_eq!(
                ret.as_ref().ok(),
                expect.as_ref(),
                "Failed: Got {:?} on {:?}",
                ret,
                s,
            );
        }
    }
}
