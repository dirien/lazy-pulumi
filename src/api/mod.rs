//! Pulumi API client module
//!
//! Provides async clients for interacting with:
//! - Pulumi Cloud REST API (stacks, resources)
//! - Pulumi ESC (environments, secrets, configs)
//! - Pulumi Neo (AI agent tasks)
//! - Pulumi Platform (services, components, templates)

mod client;
mod types;

pub use client::PulumiClient;
pub use types::{
    EscEnvironmentSummary, NeoMessage, NeoMessageType, NeoSlashCommand, NeoTask, OrgStackUpdate,
    RegistryPackage, RegistryTemplate, Resource, ResourceSummaryPoint, Service, Stack,
};
