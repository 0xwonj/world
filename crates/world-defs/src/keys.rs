use std::fmt;

macro_rules! string_key {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Creates a key from non-empty text after trimming outer whitespace.
            pub fn new(value: impl Into<String>) -> Option<Self> {
                let value = value.into();
                let value = value.trim();

                if value.is_empty() {
                    None
                } else {
                    Some(Self(value.to_owned()))
                }
            }

            /// Returns the normalized key text.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

string_key!(
    /// Runtime-facing human-readable definition name.
    DefinitionName
);

string_key!(
    /// Role name declared by an action, process, or semantic definition.
    RoleName
);

string_key!(
    /// Checked role type name.
    RoleType
);

string_key!(
    /// Checked primitive effect family name.
    EffectKind
);

string_key!(
    /// Checked event record family name.
    EventKind
);

string_key!(
    /// Checked requirement family name.
    RequirementKind
);

string_key!(
    /// Checked binding rule family name.
    BindingRuleKind
);

string_key!(
    /// Checked process policy key.
    PolicyKey
);

string_key!(
    /// Checked process state field name.
    StateFieldName
);

string_key!(
    /// Checked process state value type name.
    StateValueType
);
