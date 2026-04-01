pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn app_version() -> &'static str {
    APP_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_version_is_derived_from_cargo_package_version() {
        assert_eq!(app_version(), env!("CARGO_PKG_VERSION"));
    }
}
