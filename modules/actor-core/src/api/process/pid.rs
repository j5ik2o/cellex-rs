use alloc::{borrow::Cow, fmt, string::ToString};
use core::hash::{Hash, Hasher};

use crate::api::actor::ActorPath;

/// Identifier of the actor system namespace.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SystemId(pub Cow<'static, str>);

impl SystemId {
  /// Creates a new [`SystemId`] from the provided string.
  #[must_use]
  pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
    Self(id.into())
  }
}

impl fmt::Display for SystemId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0)
  }
}

/// Unique identifier of the node within a cluster.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId {
  host: Cow<'static, str>,
  port: Option<u16>,
}

impl NodeId {
  /// Creates a new [`NodeId`] with host and optional port.
  #[must_use]
  pub fn new(host: impl Into<Cow<'static, str>>, port: Option<u16>) -> Self {
    Self { host: host.into(), port }
  }

  /// Returns the host name.
  #[must_use]
  pub fn host(&self) -> &str {
    &self.host
  }

  /// Returns the port, if specified.
  #[must_use]
  pub fn port(&self) -> Option<u16> {
    self.port
  }
}

impl fmt::Display for NodeId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self.port {
      | Some(port) => write!(f, "{}:{}", self.host, port),
      | None => f.write_str(&self.host),
    }
  }
}

/// Optional tag associated with a PID (e.g. incarnation).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PidTag(pub Cow<'static, str>);

impl PidTag {
  /// Creates a new tag.
  #[must_use]
  pub fn new(tag: impl Into<Cow<'static, str>>) -> Self {
    Self(tag.into())
  }
}

impl fmt::Display for PidTag {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.0)
  }
}

/// Process identifier representing the global actor address.
#[derive(Clone, Debug)]
pub struct Pid {
  scheme: Cow<'static, str>,
  system: SystemId,
  node:   Option<NodeId>,
  path:   ActorPath,
  tag:    Option<PidTag>,
}

impl Pid {
  /// Creates a new PID within the specified system.
  #[must_use]
  pub fn new(system: SystemId, path: ActorPath) -> Self {
    Self { scheme: Cow::Borrowed("actor"), system, node: None, path, tag: None }
  }

  /// Assigns a node to the PID.
  #[must_use]
  pub fn with_node(mut self, node: NodeId) -> Self {
    self.node = Some(node);
    self
  }

  /// Assigns an incarnation tag to the PID.
  #[must_use]
  pub fn with_tag(mut self, tag: PidTag) -> Self {
    self.tag = Some(tag);
    self
  }

  /// Sets a custom scheme (e.g. actor+ssl).
  #[must_use]
  pub fn with_scheme(mut self, scheme: impl Into<Cow<'static, str>>) -> Self {
    self.scheme = scheme.into();
    self
  }

  /// Returns the system identifier.
  #[must_use]
  pub fn system(&self) -> &SystemId {
    &self.system
  }

  /// Returns the optional node identifier.
  #[must_use]
  pub fn node(&self) -> Option<&NodeId> {
    self.node.as_ref()
  }

  /// Returns the actor path.
  #[must_use]
  pub fn path(&self) -> &ActorPath {
    &self.path
  }

  /// Returns the optional tag.
  #[must_use]
  pub fn tag(&self) -> Option<&PidTag> {
    self.tag.as_ref()
  }

  /// Returns the scheme string.
  #[must_use]
  pub fn scheme(&self) -> &str {
    &self.scheme
  }
}

impl PartialEq for Pid {
  fn eq(&self, other: &Self) -> bool {
    self.scheme == other.scheme
      && self.system == other.system
      && self.node == other.node
      && self.path == other.path
      && self.tag == other.tag
  }
}

impl Eq for Pid {}

impl Hash for Pid {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.scheme.hash(state);
    self.system.hash(state);
    self.node.hash(state);
    self.path.to_string().hash(state);
    self.tag.hash(state);
  }
}

impl fmt::Display for Pid {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}://{}", self.scheme, self.system)?;
    if let Some(node) = &self.node {
      write!(f, "@{}", node)?;
    }
    write!(f, "{}", self.path)?;
    if let Some(tag) = &self.tag {
      write!(f, "#{}", tag)?;
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::api::actor::{ActorId, ActorPath};

  #[test]
  fn pid_display_formats_uri() {
    let mut path = ActorPath::new();
    path = path.push_child(ActorId(1));
    path = path.push_child(ActorId(2));
    let pid = Pid::new(SystemId::new("cellex"), path.clone())
      .with_node(NodeId::new("node1", Some(2552)))
      .with_tag(PidTag::new("v1"));

    assert_eq!(pid.to_string(), "actor://cellex@node1:2552/1/2#v1");
    assert_eq!(pid.path(), &path);
  }
}
