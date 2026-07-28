/// Administrative and health mode of an authoritative session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SessionMode {
    /// Scheduled work may execute.
    Running,
    /// Scheduled work is administratively suspended.
    Paused,
    /// Ordinary work is blocked pending explicit recovery.
    Quarantined,
    /// The session has entered a terminal health failure.
    Failed,
}

impl SessionMode {
    pub(crate) const fn canonical_tag(self) -> u32 {
        match self {
            Self::Running => 0,
            Self::Paused => 1,
            Self::Quarantined => 2,
            Self::Failed => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SessionMode;

    #[test]
    fn session_mode_tags_are_frozen() {
        assert_eq!(SessionMode::Running.canonical_tag(), 0);
        assert_eq!(SessionMode::Paused.canonical_tag(), 1);
        assert_eq!(SessionMode::Quarantined.canonical_tag(), 2);
        assert_eq!(SessionMode::Failed.canonical_tag(), 3);
    }
}
