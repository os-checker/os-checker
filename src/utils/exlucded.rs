use glob::Pattern;

/// Match a &str to indicate excluding the path if it returns true.
pub trait Exclude {
    fn exclude(&self, s: &str) -> bool;
}

impl Exclude for str {
    fn exclude(&self, s: &str) -> bool {
        self == s
    }
}

impl Exclude for Pattern {
    fn exclude(&self, s: &str) -> bool {
        self.matches(s)
    }
}

impl<T: ?Sized + Exclude> Exclude for &T {
    fn exclude(&self, s: &str) -> bool {
        <T as Exclude>::exclude(self, s)
    }
}

impl<T: Exclude> Exclude for [T] {
    fn exclude(&self, s: &str) -> bool {
        for t in self {
            if t.exclude(s) {
                return true;
            }
        }
        false
    }
}

impl<T: Exclude, const N: usize> Exclude for [T; N] {
    fn exclude(&self, s: &str) -> bool {
        <[T] as Exclude>::exclude(self, s)
    }
}

impl<T: Exclude> Exclude for Box<[T]> {
    fn exclude(&self, s: &str) -> bool {
        <[T] as Exclude>::exclude(self, s)
    }
}

/// An empty filter, indicating don't exclude anything matched against,
/// i.e. accept anything matched against.
pub fn empty() -> [&'static str; 0] {
    []
}

#[test]
fn test_str() {
    assert!(".github".exclude(".github"));

    assert!(!"./.github".exclude(".github"));
    assert!(!"parent/.github".exclude(".github"));
    assert!(!"github".exclude(".github"));
    assert!(!"**/github".exclude(".github"));
    assert!("*.github".exclude(".github"));
}

#[cfg(test)]
pub fn pat(s: &str) -> Pattern {
    Pattern::new(s).unwrap()
}

#[test]
fn test_pattern() {
    assert!(pat(".github").exclude(".github"));
    assert!(pat("**/.github").exclude(".github"));
    assert!(pat("*.github").exclude(".github"));

    assert!(!pat("./.github").exclude(".github"));
    assert!(!pat("parent/.github").exclude(".github"));
    assert!(!pat("github").exclude(".github"));
}

#[test]
fn test_slice() {
    let mut v_pat = vec![pat("github")];
    assert!(!v_pat.exclude(".github"));
    v_pat.push(pat("./.github"));
    assert!(!v_pat.exclude(".github"));
    v_pat.push(pat(".github"));
    assert!(v_pat.exclude(".github"));

    let mut v_str = vec!["github"];
    assert!(!v_str.as_slice().exclude(".github"));
    v_str.push(".github");
    assert!(v_str.exclude(".github"));
}
