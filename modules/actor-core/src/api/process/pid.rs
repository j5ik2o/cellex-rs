use alloc::{borrow::Cow, string::ToString};
use core::{
  fmt,
  hash::{Hash, Hasher},
  str::FromStr,
};

use crate::api::actor::{ActorId, ActorPath};

/// Errors that can occur while parsing a PID URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PidParseError {
  /// The URI is missing the `scheme://` delimiter or scheme component.
  MissingScheme,
  /// The URI does not contain a system identifier segment.
  MissingSystem,
  /// The node component contains an invalid port number.
  InvalidPort,
  /// One of the path segments could not be parsed into an [`ActorId`].
  InvalidPathSegment,
}

impl fmt::Display for PidParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      | Self::MissingScheme => f.write_str("missing scheme"),
      | Self::MissingSystem => f.write_str("missing system identifier"),
      | Self::InvalidPort => f.write_str("invalid node port"),
      | Self::InvalidPathSegment => f.write_str("invalid path segment"),
    }
  }
}

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

  /// Parses a PID from its URI representation.
  pub fn parse(input: &str) -> Result<Self, PidParseError> {
    input.parse()
  }
}

impl FromStr for Pid {
  type Err = PidParseError;

  fn from_str(input: &str) -> Result<Self, Self::Err> {
    let (scheme_part, remainder) = input.split_once("://").ok_or(PidParseError::MissingScheme)?;
    if scheme_part.is_empty() {
      return Err(PidParseError::MissingScheme);
    }

    let (before_tag, tag_part) = match remainder.split_once('#') {
      | Some((head, tail)) => (head, Some(tail)),
      | None => (remainder, None),
    };

    let (system_and_node, path_str) = match before_tag.split_once('/') {
      | Some((head, tail)) => (head, tail),
      | None => (before_tag, ""),
    };

    if system_and_node.is_empty() {
      return Err(PidParseError::MissingSystem);
    }

    let (system_str, node_opt) = match system_and_node.split_once('@') {
      | Some((system, node)) => (system, Some(node)),
      | None => (system_and_node, None),
    };

    if system_str.is_empty() {
      return Err(PidParseError::MissingSystem);
    }

    let node = node_opt
      .filter(|node| !node.is_empty())
      .map(|node_part| {
        let (host, port_str_opt) = node_part.split_once(':').map_or((node_part, None), |(h, p)| (h, Some(p)));
        let host = host.to_string();
        let port = match port_str_opt {
          | Some(port_str) if !port_str.is_empty() => {
            port_str.parse::<u16>().map(Some).map_err(|_| PidParseError::InvalidPort)
          },
          | Some(_) => Err(PidParseError::InvalidPort),
          | None => Ok(None),
        }?;
        Ok(NodeId::new(host, port))
      })
      .transpose()?;

    let mut path = ActorPath::new();
    for segment in path_str.split('/') {
      if segment.is_empty() {
        continue;
      }
      let id = segment.parse::<usize>().map_err(|_| PidParseError::InvalidPathSegment)?;
      path = path.push_child(ActorId(id));
    }

    let tag = tag_part.filter(|tag_str| !tag_str.is_empty()).map(|tag_str| PidTag::new(tag_str.to_string()));

    Ok(Pid {
      scheme: Cow::Owned(scheme_part.to_string()),
      system: SystemId::new(system_str.to_string()),
      node,
      path,
      tag,
    })
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
mod tests;
