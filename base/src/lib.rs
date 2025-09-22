pub mod error;
pub mod indent;
pub mod result;
pub mod value;

pub fn unansi(string: &str) -> String {
    anstream::adapter::strip_str(string).to_string()
}
