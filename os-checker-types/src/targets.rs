use crate::prelude::*;

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct Targets {
    pub inner: Vec<TargetInner>,
}

#[derive(Debug, Serialize)]
pub struct TargetInner {
    pub triple: String,
    pub arch: XString,
}
