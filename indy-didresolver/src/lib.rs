pub mod did_document;
pub mod resolver;
pub mod responses;
pub mod did;
pub mod error;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
