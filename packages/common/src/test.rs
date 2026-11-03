// Tests may use plain arithmetic and unwrap; the workspace lints target
// contract code (README §5).
#![allow(clippy::arithmetic_side_effects, clippy::unwrap_used, clippy::panic)]

use crate::amount;

#[test]
fn rejects_non_positive_amounts() {
    assert!(amount::is_positive(1));
    assert!(!amount::is_positive(0));
    assert!(!amount::is_positive(-1));
}
