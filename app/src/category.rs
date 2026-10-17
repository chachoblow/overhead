//! Curated display roles from valid source-group membership (decision 0045).
//! Not mission inference, operational status, or orbital-record selection.

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    #[default]
    Unknown,
    Weather,
    Navigation,
    Station,
}

impl Category {
    fn priority(self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::Weather => 1,
            Self::Navigation => 2,
            Self::Station => 3,
        }
    }

    /// Highest display priority among retained valid memberships. The table is
    /// indexed by catalogue group index, not by selected element provenance.
    /// Missing entries contribute no classification, just like Unknown. No
    /// name/URL/ID inference, allocation, or manifest-order tie breaking.
    pub fn from_groups(groups: &[usize], categories: &[Self]) -> Self {
        groups
            .iter()
            .filter_map(|&index| categories.get(index).copied())
            .max_by_key(|category| category.priority())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::Category::{self, *};

    #[test]
    fn hierarchy_is_explicit_and_order_duplicate_and_unknown_independent() {
        for a in [Unknown, Weather, Navigation, Station] {
            for b in [Unknown, Weather, Navigation, Station] {
                let expected = if [a, b].contains(&Station) {
                    Station
                } else if [a, b].contains(&Navigation) {
                    Navigation
                } else if [a, b].contains(&Weather) {
                    Weather
                } else {
                    Unknown
                };
                for groups in [&[0, 1][..], &[1, 0, 1, 99][..]] {
                    assert_eq!(Category::from_groups(groups, &[a, b]), expected);
                    assert_eq!(Category::from_groups(groups, &[b, a]), expected);
                }
            }
        }
        assert_eq!(Category::from_groups(&[], &[Station]), Unknown);
        assert_eq!(Category::from_groups(&[0, usize::MAX], &[]), Unknown);
        assert_eq!(Category::from_groups(&[1], &[Station, Weather]), Weather);
    }
}
