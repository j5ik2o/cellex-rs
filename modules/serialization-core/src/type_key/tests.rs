use super::*;
use crate::{impl_type_key, SerializerId};

struct Basic;

impl_type_key!(Basic);

struct Custom;

impl_type_key!(Custom, "custom.Type");

struct WithSerializer;

impl_type_key!(WithSerializer, "custom.Serializer", SerializerId::new(99));

#[test]
fn derives_type_name_by_default() {
  assert_eq!(<Basic as TypeKey>::type_key(), core::any::type_name::<Basic>());
  assert_eq!(<Basic as TypeKey>::default_serializer(), None);
}

#[test]
fn overrides_key_when_specified() {
  assert_eq!(<Custom as TypeKey>::type_key(), "custom.Type");
}

#[test]
fn exposes_default_serializer_when_requested() {
  assert_eq!(<WithSerializer as TypeKey>::default_serializer(), Some(SerializerId::new(99)));
}
