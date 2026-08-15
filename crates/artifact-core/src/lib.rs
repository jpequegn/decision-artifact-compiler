//! Core contracts for approved decision artifacts.

/// Supported decision artifact format version.
pub const ARTIFACT_FORMAT_VERSION: &str = "v1";

#[must_use]
pub const fn artifact_format_version() -> &'static str {
    ARTIFACT_FORMAT_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_format_version() {
        assert_eq!(artifact_format_version(), "v1");
    }
}
