use core::num::NonZeroU64;

/// World revision coordinate, including the unpublished root revision.
///
/// The sealed runtime record that carries a value establishes publication
/// authority; this scalar proves only its numeric representation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldRevision(u64);

impl WorldRevision {
    /// Revision of an initial root before any authority record is published.
    pub const ROOT: Self = Self(0);

    /// Creates a revision coordinate decoded from an enclosing protocol value.
    #[must_use]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the numeric revision.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Returns the next nonzero revision coordinate, or `None` on overflow.
    #[must_use]
    pub const fn checked_next(self) -> Option<NonZeroWorldRevision> {
        match self.0.checked_add(1) {
            Some(value) => match NonZeroU64::new(value) {
                Some(value) => Some(NonZeroWorldRevision(value)),
                None => None,
            },
            None => None,
        }
    }
}

/// Nonzero world revision coordinate.
///
/// This scalar proves nonzero shape only. A sealed runtime record establishes
/// whether the represented revision was actually published.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonZeroWorldRevision(NonZeroU64);

impl NonZeroWorldRevision {
    /// Creates a published revision when `value` is nonzero.
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        match NonZeroU64::new(value) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    /// Returns the numeric revision.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    /// Returns the preceding revision.
    #[must_use]
    pub const fn previous(self) -> WorldRevision {
        WorldRevision::from_raw(self.get() - 1)
    }

    /// Returns the next nonzero revision coordinate, or `None` on overflow.
    #[must_use]
    pub const fn checked_next(self) -> Option<Self> {
        WorldRevision::from_raw(self.get()).checked_next()
    }
}

impl From<NonZeroWorldRevision> for WorldRevision {
    fn from(value: NonZeroWorldRevision) -> Self {
        Self::from_raw(value.get())
    }
}
