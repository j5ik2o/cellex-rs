#![allow(clippy::disallowed_types)]
#![allow(clippy::unwrap_used)]

use super::{NodeId, Pid, PidParseError, PidTag, SystemId};
use crate::{
  alloc::string::ToString,
  api::actor::{ActorId, ActorPath},
};

fn actor_path(ids: &[usize]) -> ActorPath {
  ids.iter().fold(ActorPath::new(), |path, id| path.push_child(ActorId(*id)))
}

#[test]
fn pid_display_formats_uri() {
  let path = actor_path(&[1, 2]);
  let pid = Pid::new(SystemId::new("cellex"), path.clone())
    .with_node(NodeId::new("node1", Some(2552)))
    .with_tag(PidTag::new("v1"));

  assert_eq!(pid.to_string(), "actor://cellex@node1:2552/1/2#v1");
  assert_eq!(pid.path(), &path);
}

#[test]
fn parse_pid_from_uri() {
  let pid = Pid::parse("actor://cellex@node1:2552/1/2#v1").expect("parse pid");
  assert_eq!(pid.scheme(), "actor");
  assert_eq!(pid.system().to_string(), "cellex");
  assert_eq!(pid.node().unwrap().host(), "node1");
  assert_eq!(pid.node().unwrap().port(), Some(2552));
  assert_eq!(pid.tag().unwrap().to_string(), "v1");
  assert_eq!(pid.path(), &actor_path(&[1, 2]));
}

#[test]
fn parse_pid_without_node_or_tag() {
  let pid = Pid::parse("actor://cellex/42").expect("parse pid");
  assert!(pid.node().is_none());
  assert!(pid.tag().is_none());
  assert_eq!(pid.path(), &actor_path(&[42]));
}

#[test]
fn parse_pid_invalid_segment_fails() {
  let err = Pid::parse("actor://cellex/foo").expect_err("should fail");
  assert_eq!(err, PidParseError::InvalidPathSegment);
}

#[test]
fn parse_pid_invalid_port_fails() {
  let err = Pid::parse("actor://cellex@node:notaport/1").expect_err("should fail");
  assert_eq!(err, PidParseError::InvalidPort);
}
