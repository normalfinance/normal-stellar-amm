pub fn log10(n: u128) -> u128 {
    if n < 10 { 0 } else if n == 10 { 1 } else { log10(n / 10) + 1 }
}

pub fn log10_iter(n: u128) -> u128 {
    let mut result = 0;
    let mut n_copy = n;

    while n_copy >= 10 {
        result += 1;
        n_copy /= 10;
    }

    result
}
