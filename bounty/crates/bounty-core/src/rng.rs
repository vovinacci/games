//! The random numbers of the prototype rules, `prototype-lcg32-v1`: a 32-bit linear congruential generator, written
//! out here so every build draws the same numbers. Changing anything in this file changes the ruleset.

/// The generator. Its whole state is one number, which a save file stores as it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lcg32(u32);

impl Lcg32 {
    /// A generator in the given state. Zero is a valid state.
    #[must_use]
    pub const fn new(state: u32) -> Self {
        Self(state)
    }

    /// The state, for saving and for digests.
    #[must_use]
    pub const fn state(self) -> u32 {
        self.0
    }

    /// Moves on one step and returns the new state.
    pub const fn next_raw(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }

    /// A number from `lo` to `hi`, both included, every one equally likely: raw draws that would favour the low
    /// numbers are thrown away and drawn again. When `lo == hi` nothing is drawn. A `hi` below `lo` counts as `lo`;
    /// content loading refuses such ranges before they get here.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        if hi <= lo {
            return lo;
        }
        let span = u64::from(hi - lo) + 1;
        let all = 1_u64 << 32;
        let limit = all - all % span;
        loop {
            let raw = u64::from(self.next_raw());
            if raw < limit {
                return lo + (raw % span) as u32;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_draws_match_the_recorded_vector() {
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("../../../prototype/expected_cases.json")).unwrap();
        let vector = &expected["rng_vector"];
        let mut rng = Lcg32::new(u32::try_from(vector["initial_state"].as_u64().unwrap()).unwrap());
        for want in vector["first_five_raw_draws"].as_array().unwrap() {
            assert_eq!(u64::from(rng.next_raw()), want.as_u64().unwrap());
        }
    }

    #[test]
    fn a_range_of_one_number_draws_nothing() {
        let mut rng = Lcg32::new(7);
        assert_eq!(rng.range(5, 5), 5);
        assert_eq!(rng.state(), 7);
    }

    #[test]
    fn a_range_stays_inside_its_bounds_and_reaches_both_ends() {
        let mut rng = Lcg32::new(1);
        let draws: Vec<u32> = (0..200).map(|_| rng.range(2, 4)).collect();
        assert!(draws.iter().all(|n| (2..=4).contains(n)));
        assert!(draws.contains(&2) && draws.contains(&4));
    }

    #[test]
    fn the_widest_range_does_not_overflow() {
        let mut rng = Lcg32::new(1);
        let first = Lcg32::new(1).next_raw();
        assert_eq!(rng.range(0, u32::MAX), first);
    }
}
