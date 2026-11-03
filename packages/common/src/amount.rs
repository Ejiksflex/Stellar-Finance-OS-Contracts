//! Amount validation shared by contracts. All token amounts are `i128`
//! (Stellar token interface) and must be strictly positive.

/// `true` if `amount` is a valid, strictly positive token amount.
pub const fn is_positive(amount: i128) -> bool {
    amount > 0
}
