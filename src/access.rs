macro_rules! access {
    ($field_name:ident) => {
        |x| x.$field_name
    };
}

struct Test {
    a: u8,
    beta: u8,
    maybe: Option<String>,
}

fn test() {
    let s = Test {
        a: 0,
        beta: 1,
        maybe: None,
    };

    let b = Option::from(s);
    let c = b.map(access!(a)).unwrap();

    println!("{c}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        test()
    }
}
