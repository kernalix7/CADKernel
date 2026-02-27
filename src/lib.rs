/// Application name used in banners and user-facing output.
pub const APP_NAME: &str = "CADKernel";

/// Returns a formatted version banner string for CLI/startup display.
pub fn version_banner(version: &str) -> String {
    format!("{APP_NAME} v{version} - pre-alpha")
}

#[cfg(test)]
mod tests {
    use super::version_banner;

    #[test]
    fn version_banner_contains_version() {
        let text = version_banner("0.1.0");
        assert!(text.contains("0.1.0"));
    }
}
