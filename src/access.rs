#[allow(unused_macros)]
macro_rules! access {
    ($field_name:ident) => {
        |x| x.$field_name
    };
}

#[cfg(test)]
mod tests {
    struct Test {
        a: u8,
        _beta: u8,
        _maybe: Option<String>,
    }

    fn test() {
        let s = Test {
            a: 0,
            _beta: 1,
            _maybe: None,
        };

        let b = Option::from(s);
        let c = b.map(access!(a)).unwrap();

        println!("{c}")
    }

    #[test]
    fn test_name() {
        test()
    }
}
