#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

/// Returns the canonical hello-world greeting.
pub fn hello() -> &'static str {
    "Hello, world!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_returns_greeting() {
        assert_eq!(hello(), "Hello, world!");
    }
}
