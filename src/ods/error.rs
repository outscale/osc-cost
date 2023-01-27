// Copyright 2018 Serde Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use serde::ser;
use std::fmt::{self, Display};

pub type Result<T> = std::result::Result<T, Error>;

// This is a bare-bones implementation. A real library would provide additional
// information in its error type, for example the line and column at which the
// error occurred, the byte offset into the input, or the current key being
// processed.
#[derive(Debug)]
pub enum Error {
    Message(String),
    ExpectedStartStruct,
    ExpectedEndStruct,
    UnsupportedValue { kind: String },
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => write!(f, "{msg}"),
            Error::ExpectedStartStruct => {
                f.write_str("unexpected value type to start serialization")
            }
            Error::ExpectedEndStruct => {
                f.write_str("expect an end of the struct before starting new one")
            }
            Error::UnsupportedValue { kind } => f.write_str(&format!(
                "unsupported value encountered while serializing: {kind}"
            )),
        }
    }
}

impl std::error::Error for Error {}
