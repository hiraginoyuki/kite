use std::ops::{Bound::*, RangeBounds};
use Ranging::*;

pub enum Ranging {
    LessThanStart = -1,
    Contained = 0,
    GreaterThanEnd = 1,
}

pub trait RangeCmp<T: PartialOrd>: RangeBounds<T> {
    fn ranging(&self, other: &T) -> Ranging {
        if self.contains(other) {
            return Contained;
        }

        //     (unreachable if Contained) & reached
        //  => !Contained
        // <=> LessThanStart | GreaterThanEnd

        // match {
        //  (a) !HasStart & !HasEnd                   => unreachable!(),
        //  (b) !HasStart &  HasEnd                   => GreaterThanEnd,
        //  (c)  HasStart & !HasEnd                   => LessThanStart,
        //  (d1) HasStart &  HasEnd if value <= lower => LessThanStart
        //  (d2) HasStart &  HasEnd if upper <= value => GreaterThanEnd
        // }

        match self.start_bound() {
            // c | d1
            Included(bound) | Excluded(bound) if other <= bound => LessThanStart,

            // b | d2
            _ => GreaterThanEnd,
        }
    }
}

impl<T, Range> RangeCmp<T> for Range
where
    T: PartialOrd,
    Range: RangeBounds<T>,
{
}
