use once_cell::sync::Lazy;
use regex::Regex;

static EXCLUDED_SYMBOLS: &[&str] = &[
    "__libc_start_main",
    "main",
    "abort",
    "cachectl",
    "cacheflush",
    "puts",
    "atol",
    "malloc_trim",
];

static EXCLUDED_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    [r"^[_\.].*$", r"^.*64$", r"^str.*$", r"^mem.*$"]
        .into_iter()
        .map(|pattern| Regex::new(pattern).expect("invalid built-in regex"))
        .collect()
});

pub fn should_exclude(symbol: &str) -> bool {
    EXCLUDED_SYMBOLS.contains(&symbol)
        || EXCLUDED_PATTERNS
            .iter()
            .any(|pattern| pattern.is_match(symbol))
}

#[cfg(test)]
mod tests {
    use super::should_exclude;

    #[test]
    fn excludes_fixed_symbols() {
        assert!(should_exclude("main"));
        assert!(should_exclude("malloc_trim"));
    }

    #[test]
    fn excludes_pattern_based_symbols() {
        assert!(should_exclude("__init"));
        assert!(should_exclude("strcpy"));
        assert!(should_exclude("memcpy"));
        assert!(should_exclude("foo64"));
        assert!(!should_exclude("connect"));
    }
}
