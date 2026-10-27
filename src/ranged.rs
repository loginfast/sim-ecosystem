use nutype::nutype;

#[nutype(
    validate(finite, greater_or_equal = 0, less_or_equal = 1),
    derive(Clone, Copy, PartialEq, Eq, PartialOrd, Display)
)]
pub struct UnitInterval(f64);

#[nutype(
    validate(finite, greater_or_equal = -1, less_or_equal = 1),
    derive(Clone, Copy, PartialEq, Eq, PartialOrd, Display)
)]
pub struct SignedUnitInterval(f64);
