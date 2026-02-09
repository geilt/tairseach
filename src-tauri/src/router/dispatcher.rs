//! Capability Router Dispatcher
//!
//! Main entry point for manifest-based routing.

use std::sync::Arc;

use crate::auth::AuthBroker;
use crate::manifest::ManifestRegistry;

/// The capability router
pub struct CapabilityRouter {
    pub(super) registry: Arc<ManifestRegistry>,
    pub(super) auth_broker: Arc<AuthBroker>,
}
