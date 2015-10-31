use std::cmp;

fn binomial(n: isize, k: isize) -> u64 {
    // TODO: maybe lookup table?!
    (0..cmp::min(k, n-k))
        .fold(1, |n_acc, i| (n_acc * ((n-i) as u64)) / ((i+1) as u64))
}

#[test]
fn test_binomial() {
    for n in 1..10 {
        assert!(binomial(n, 1)==(n as u64));
    }
    assert!(binomial(5,2)==10);
    for n in 1..10 {
        for k in 0..n+1 {
            assert!(binomial(n, k)==binomial(n, n-k));
        }
    }
}
