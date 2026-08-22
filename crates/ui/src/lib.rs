use gpui::{App, Entity, Window};
use gpui_component::Root;

pub(crate) use cadence_core::{calendar, domain, editor, store};

mod app;
mod components;

/// Canonical human-readable application name.
pub const APPLICATION_NAME: &str = "Cadence";

/// Canonical reverse-DNS application identifier used by desktop environments.
pub const APPLICATION_ID: &str = "io.github.arapsum.Cadence";

/// Build and dependency identity displayed by support surfaces and the CLI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuildInfo {
    /// Version from the workspace package metadata.
    pub version: &'static str,
    /// Release commit supplied by CI, or `development` for local builds.
    pub commit: &'static str,
    /// GPUI revision used by the workspace.
    pub gpui_revision: &'static str,
}

impl BuildInfo {
    /// Returns the build identity compiled into this application.
    #[must_use]
    pub const fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            commit: match option_env!("CADENCE_BUILD_COMMIT") {
                Some(commit) => commit,
                None => "development",
            },
            gpui_revision: "2b37a3ed5ec75a54f67936630548da03d411d2e8",
        }
    }
}

/// Initializes Cadence's GPUI components, themes, and application actions.
///
/// # Parameters
///
/// - `cx`: Application context receiving Cadence's global registrations.
pub fn init(cx: &mut App) {
    gpui_component::init(cx);
    app::init(cx);
}

/// Creates Cadence's root entity and installs its save-aware close behavior.
///
/// # Parameters
///
/// - `window`: Window receiving the Cadence root view.
/// - `cx`: Application context used to create the root entity.
///
/// # Returns
///
/// The `gpui_component` root that renders the Cadence workspace.
pub fn mount(window: &mut Window, cx: &mut App) -> Entity<Root> {
    app::mount(window, cx)
}

#[cfg(test)]
mod tests {
    use super::{APPLICATION_ID, APPLICATION_NAME, BuildInfo};

    #[test]
    fn build_identity_is_populated() {
        let build = BuildInfo::current();

        assert_eq!(APPLICATION_NAME, "Cadence");
        assert_eq!(APPLICATION_ID, "io.github.arapsum.Cadence");
        assert!(!build.version.is_empty());
        assert!(!build.commit.is_empty());
        assert_eq!(build.gpui_revision.len(), 40);
    }
}
