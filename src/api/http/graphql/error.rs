#[derive(Debug, thiserror::Error)]
#[error("Unknown provider")]
pub struct UnknownProvider;

#[derive(Debug, thiserror::Error)]
#[error("Instance is gone")]
pub struct InstanceGone;
