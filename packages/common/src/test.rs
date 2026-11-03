use crate::amount;

#[test]
fn rejects_non_positive_amounts() {
    assert!(amount::is_positive(1));
    assert!(!amount::is_positive(0));
    assert!(!amount::is_positive(-1));
}
