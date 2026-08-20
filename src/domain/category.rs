use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ValidationError;

/// Stable identifier for a category.
#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CategoryId(Uuid);

impl CategoryId {
    /// Creates a new time-ordered category identifier.
    ///
    /// # Returns
    ///
    /// A new category identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Wraps an existing `Uuid` as a category identifier.
    ///
    /// # Parameters
    ///
    /// - `id`: UUID value to wrap.
    ///
    /// # Returns
    ///
    /// A category identifier backed by `id`.
    #[must_use]
    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Returns the underlying `Uuid` value.
    ///
    /// # Returns
    ///
    /// The UUID stored by this identifier.
    #[must_use]
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for CategoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Uuid> for CategoryId {
    fn from(value: Uuid) -> Self {
        Self::from_uuid(value)
    }
}

impl FromStr for CategoryId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self::from_uuid)
    }
}

impl fmt::Display for CategoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Semantic category colors. Rendering maps these tokens to the active theme.
#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum CategoryColor {
    Lime,
    Yellow,
    Coral,
    Violet,
    Cyan,
    Blue,
}

/// A named category used to organize timetable events.
///
/// # Fields
///
/// - `id`: Stable identifier for the category.
/// - `name`: User-facing category name.
/// - `color_token`: Semantic color token used by calendar surfaces.
/// - `is_visible`: Whether events in this category are shown.
#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct Category {
    id: CategoryId,
    name: String,
    color_token: CategoryColor,
    is_visible: bool,
}

impl Category {
    /// Creates a validated category.
    ///
    /// # Parameters
    ///
    /// - `id`: Stable identifier for the category.
    /// - `name`: User-facing category name.
    /// - `color_token`: Semantic color token used by calendar surfaces.
    /// - `is_visible`: Whether events in this category are shown.
    ///
    /// # Returns
    ///
    /// A category with trimmed text values.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// - `name` is empty after trimming.
    pub fn new(
        id: CategoryId,
        name: impl Into<String>,
        color_token: CategoryColor,
        is_visible: bool,
    ) -> Result<Self, ValidationError> {
        let name = name.into().trim().to_owned();
        if name.is_empty() {
            return Err(ValidationError::EmptyCategoryName);
        }

        Ok(Self {
            id,
            name,
            color_token,
            is_visible,
        })
    }

    /// Returns the category identifier.
    #[must_use]
    pub const fn id(&self) -> CategoryId {
        self.id
    }

    /// Returns the user-facing category name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the semantic color token.
    #[must_use]
    pub const fn color_token(&self) -> CategoryColor {
        self.color_token
    }

    /// Returns whether the category is visible.
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.is_visible
    }

    /// Changes whether the category is visible.
    ///
    /// # Parameters
    ///
    /// - `is_visible`: New visibility state.
    pub const fn set_visible(&mut self, is_visible: bool) {
        self.is_visible = is_visible;
    }
}
