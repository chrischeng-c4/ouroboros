//! `ob api g` command implementation
//!
//! Shorthand commands for common code generation operations.
//! These are aliases for the more verbose commands.

use anyhow::Result;

use super::{GenerateAction, app, feat, core, test};

/// Execute a generate action
pub async fn execute(action: GenerateAction) -> Result<()> {
    match action {
        GenerateAction::App { name, port } => {
            let args = app::CreateArgs {
                name,
                port,
                description: None,
            };
            app::execute(super::AppAction::Create(args)).await
        }
        GenerateAction::Feature { name, db } => {
            let args = feat::CreateArgs { name, db };
            feat::execute(super::FeatAction::Create(args)).await
        }
        GenerateAction::Route { module, app, core: is_core } => {
            if is_core {
                let args = core::RouteArgs { module, app: Some(app), model: None, fields: None };
                core::execute(super::CoreAction::Route(args)).await
            } else {
                let args = feat::RouteArgs { module, app: Some(app), model: None, fields: None };
                feat::execute(super::FeatAction::Route(args)).await
            }
        }
        GenerateAction::Test(args) => {
            test::execute(args).await
        }
    }
}
