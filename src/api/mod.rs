//! Pulumi API client module
//!
//! Provides async clients for interacting with:
//! - Pulumi Cloud REST API (stacks, resources)
//! - Pulumi ESC (environments, secrets, configs)
//! - Pulumi Neo (AI agent tasks)
//! - Pulumi Platform (services, components, templates)

mod client;
mod client_esc;
mod client_neo;
mod client_platform;
mod client_stacks;
mod convert;
mod domain;
mod generated;

pub use client::{ApiError, PulumiClient};
pub use domain::{
    EscEnvironmentSummary, NeoMessage, NeoMessageType, NeoSlashCommand, NeoTask, OrgStackUpdate,
    RegistryPackage, RegistryTemplate, Resource, ResourceSummaryPoint, Service, Stack,
};
