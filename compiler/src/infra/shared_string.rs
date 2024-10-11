use arcstr::ArcStr;

pub type SharedString = ArcStr;
pub type Name = SharedString;

#[cfg(test)]
mod tests {
    use crate::infra::shared_string::SharedString;

    #[test]
    fn test_eq() {
        let s = SharedString::from("foo");
        assert_eq!("foo", s);
    }

    #[test]
    fn test_display() {
        let s = SharedString::from("foo");
        assert_eq!("foo", format!("{}", s));
    }

    #[test]
    fn test_debug() {
        let s = SharedString::from("foo");
        assert_eq!("\"foo\"", format!("{:?}", s));
    }

    #[test]
    fn test_clone() {
        let s = SharedString::from("foo");
        let s2 = s.clone();
        assert_eq!("foo", format!("{}", s2));
        assert_eq!(s, s2);
    }

    #[test]
    fn test_str_use() {
        fn f(input: &str) -> &str {
            input
        }
        let s = SharedString::from("foo");
        let s2 = f(&s);
        assert_eq!("foo", format!("{}", s2));
        assert_eq!(s, s2);
    }
}
