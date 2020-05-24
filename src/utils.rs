pub fn trim_trailing_slash(s: &str) -> String {
    s.trim_end_matches("/").into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        assert_eq!(
            trim_trailing_slash("http://example.com/"),
            "http://example.com"
        );
        assert_eq!(
            trim_trailing_slash("http://example.com/a"),
            "http://example.com/a"
        );
    }

    #[test]
    fn multiple() {
        assert_eq!(
            trim_trailing_slash("http://example.com///"),
            "http://example.com"
        );
    }

    #[test]
    fn none() {
        assert_eq!(
            trim_trailing_slash("http://example.com"),
            "http://example.com"
        );
    }

    #[test]
    fn empty() {
        assert_eq!(trim_trailing_slash(""), "");
    }
}
