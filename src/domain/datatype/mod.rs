pub mod security;

use crate::error::resource::ValidationFieldError;

// ### JsonPointer

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonPointer {
    segments: Vec<Box<str>>,
}

impl JsonPointer {
    // fn as_uri_fragment(&self) -> String {}
}

impl std::str::FromStr for JsonPointer {
    type Err = ValidationFieldError;

    fn from_str(_: &str) -> Result<Self, Self::Err> {
        todo!("https://www.rfc-editor.org/rfc/rfc6901#section-4")
    }
}

impl std::fmt::Display for JsonPointer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for segment in self.segments.iter() {
            write!(f, "/{}", segment)?;
        }
        Ok(())
    }
}
