pub(crate) mod b1248;
pub(crate) mod s1144;

/// Badge type
#[derive(Debug, PartialEq, Copy, Clone)]
#[allow(dead_code)]
pub enum BadgeType {
    S1144 = 0,
    B1248,
}
