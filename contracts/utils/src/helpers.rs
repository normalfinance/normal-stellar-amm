pub fn log10_iter(n: u128) -> u128 {
    let mut result = 0;
    let mut n_copy = n;

    while n_copy >= 10 {
        result += 1;
        n_copy /= 10;
    }

    result
}
