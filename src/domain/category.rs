use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ValidationError;

/// Stable identifier for a category.
#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct CategoryId(Uuid);

impl CategoryId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub const fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

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

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct Category {
    id: CategoryId,
    name: String,
    color_token: CategoryColor,
    is_visible: bool,
}

impl Category {
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

    pub fn id(&self) -> CategoryId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn color_token(&self) -> CategoryColor {
        self.color_token
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    pub fn set_visible(&mut self, is_visible: bool) {
        self.is_visible = is_visible;
    }
}
