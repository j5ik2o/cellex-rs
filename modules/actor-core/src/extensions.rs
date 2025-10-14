#![cfg(feature = "alloc")]

#[cfg(test)]
mod tests;

#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as SharedArc;
use alloc::string::String;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc as SharedArc;
use alloc::vec::Vec;
use core::any::Any;
use core::fmt::{self, Debug, Formatter};
use portable_atomic::{AtomicI32, Ordering};

use cellex_serialization_core_rs::id::SerializerId;
use cellex_serialization_core_rs::registry::InMemorySerializerRegistry;
use cellex_serialization_core_rs::serializer::Serializer;
use cellex_serialization_core_rs::{BindingError, RegistryError, SerializationRouter, TypeBindingRegistry, TypeKey};
#[cfg(feature = "std")]
use cellex_serialization_json_rs::{shared_json_serializer, JsonTypeKey, SERDE_JSON_SERIALIZER_ID};
#[cfg(feature = "postcard")]
use cellex_serialization_postcard_rs::{shared_postcard_serializer, PostcardTypeKey, POSTCARD_SERIALIZER_ID};
#[cfg(feature = "std")]
use cellex_serialization_prost_rs::{shared_prost_serializer, ProstTypeKey, PROST_SERIALIZER_ID};
use cellex_utils_core_rs::sync::{ArcShared, SharedBound};
use spin::RwLock;

/// 一意な Extension を識別するための ID 型。
pub type ExtensionId = i32;

static NEXT_EXTENSION_ID: AtomicI32 = AtomicI32::new(0);
static SERIALIZER_EXTENSION_ID: AtomicI32 = AtomicI32::new(-1);

/// 新しい ExtensionId を払い出します。
#[must_use]
pub fn next_extension_id() -> ExtensionId {
  NEXT_EXTENSION_ID.fetch_add(1, Ordering::SeqCst)
}

fn acquire_serializer_extension_id() -> ExtensionId {
  let current = SERIALIZER_EXTENSION_ID.load(Ordering::SeqCst);
  if current >= 0 {
    return current;
  }
  let new_id = next_extension_id();
  match SERIALIZER_EXTENSION_ID.compare_exchange(-1, new_id, Ordering::SeqCst, Ordering::SeqCst) {
    Ok(_) => new_id,
    Err(existing) => existing,
  }
}

/// Serializer 拡張の予約済み ExtensionId を取得します。
#[must_use]
pub fn serializer_extension_id() -> ExtensionId {
  acquire_serializer_extension_id()
}

/// ActorSystem に組み込まれる拡張の共通インターフェース。
pub trait Extension: Any + SharedBound {
  /// 拡張固有の ID を返します。
  fn extension_id(&self) -> ExtensionId;

  /// Returns a type-erased `Any` reference for downcasting.
  fn as_any(&self) -> &dyn Any;
}

/// 登録済み Extension 群を管理するスロットコンテナ。
pub struct Extensions {
  slots: ArcShared<RwLock<Vec<Option<ArcShared<dyn Extension>>>>>,
}

impl Extensions {
  /// 空の Extension レジストリを生成します。
  #[must_use]
  pub fn new() -> Self {
    Self {
      slots: ArcShared::new(RwLock::new(Vec::new())),
    }
  }

  /// 既存スロット共有体からレジストリを作成します。
  #[must_use]
  pub fn from_shared(slots: ArcShared<RwLock<Vec<Option<ArcShared<dyn Extension>>>>>) -> Self {
    Self { slots }
  }

  /// 型付き Extension を登録します。
  pub fn register<E>(&self, extension: ArcShared<E>)
  where
    E: Extension + 'static, {
    let id = extension.extension_id();
    if id < 0 {
      return;
    }
    let idx = id as usize;
    let arc: SharedArc<E> = extension.into_arc();
    let trait_arc: SharedArc<dyn Extension> = arc;
    let handle = ArcShared::from_arc(trait_arc);
    let mut guard = self.slots.write();
    if guard.len() <= idx {
      guard.resize_with(idx + 1, || None);
    }
    guard[idx] = Some(handle);
  }

  /// Trait オブジェクトとして Extension を登録します。
  pub fn register_dyn(&self, extension: ArcShared<dyn Extension>) {
    let id = extension.extension_id();
    if id < 0 {
      return;
    }
    let idx = id as usize;
    let mut guard = self.slots.write();
    if guard.len() <= idx {
      guard.resize_with(idx + 1, || None);
    }
    guard[idx] = Some(extension);
  }

  /// 指定 ID の Extension を取得します。
  #[must_use]
  pub fn get(&self, id: ExtensionId) -> Option<ArcShared<dyn Extension>> {
    if id < 0 {
      return None;
    }
    let guard = self.slots.read();
    guard.get(id as usize).and_then(|slot| slot.as_ref().cloned())
  }

  /// 指定 ID の Extension に対してクロージャを適用します。
  pub fn with<E, F, R>(&self, id: ExtensionId, f: F) -> Option<R>
  where
    E: Extension + 'static,
    F: FnOnce(&E) -> R, {
    let guard = self.slots.read();
    guard.get(id as usize).and_then(|slot| {
      slot
        .as_ref()
        .and_then(|handle| (*handle).as_any().downcast_ref::<E>().map(f))
    })
  }
}

impl Default for Extensions {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for Extensions {
  fn clone(&self) -> Self {
    Self {
      slots: self.slots.clone(),
    }
  }
}

impl Debug for Extensions {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    let guard = self.slots.read();
    f.debug_struct("Extensions").field("len", &guard.len()).finish()
  }
}

/// Extension that exposes the shared serializer registry.
pub struct SerializerRegistryExtension {
  id: ExtensionId,
  registry: InMemorySerializerRegistry,
  bindings: TypeBindingRegistry,
  router: SerializationRouter,
}

impl SerializerRegistryExtension {
  /// Creates a new registry extension and installs built-in serializers.
  #[must_use]
  pub fn new() -> Self {
    let registry = InMemorySerializerRegistry::new();
    let bindings = TypeBindingRegistry::new();
    let router = SerializationRouter::new(bindings.clone(), registry.clone());
    let extension = Self {
      id: serializer_extension_id(),
      registry,
      bindings,
      router,
    };
    extension.install_builtin_serializers();
    extension.install_default_bindings();
    extension
  }

  fn install_builtin_serializers(&self) {
    #[cfg(feature = "std")]
    {
      if self.registry.get(SERDE_JSON_SERIALIZER_ID).is_none() {
        let serializer = shared_json_serializer();
        let _ = self.registry.register(serializer);
      }
      if self.registry.get(PROST_SERIALIZER_ID).is_none() {
        let serializer = shared_prost_serializer();
        let _ = self.registry.register(serializer);
      }
    }
    #[cfg(feature = "postcard")]
    {
      if self.registry.get(POSTCARD_SERIALIZER_ID).is_none() {
        let serializer = shared_postcard_serializer();
        let _ = self.registry.register(serializer);
      }
    }
  }

  fn install_default_bindings(&self) {
    #[cfg(feature = "std")]
    {
      let _ = self.bind_type::<JsonTypeKey>(SERDE_JSON_SERIALIZER_ID);
      let _ = self.bind_type::<ProstTypeKey>(PROST_SERIALIZER_ID);
    }
    #[cfg(feature = "postcard")]
    {
      let _ = self.bind_type::<PostcardTypeKey>(POSTCARD_SERIALIZER_ID);
    }
  }

  /// Returns a reference to the underlying registry.
  #[must_use]
  pub fn registry(&self) -> &InMemorySerializerRegistry {
    &self.registry
  }

  /// Returns the binding registry used by the router.
  #[must_use]
  pub fn bindings(&self) -> &TypeBindingRegistry {
    &self.bindings
  }

  /// Returns a serialization router instance backed by the shared registries.
  #[must_use]
  pub fn router(&self) -> SerializationRouter {
    self.router.clone()
  }

  /// Registers a serializer implementation, returning an error when the ID clashes.
  pub fn register_serializer<S>(&self, serializer: ArcShared<S>) -> Result<(), RegistryError>
  where
    S: Serializer + 'static, {
    self.registry.register(serializer)
  }

  /// Binds the provided key to the specified serializer identifier.
  pub fn bind_key<K>(&self, key: K, serializer: SerializerId) -> Result<(), BindingError>
  where
    K: Into<String>, {
    self.bindings.bind(key, serializer)
  }

  /// Binds the [`TypeKey::KEY`] of `T` to the specified serializer identifier.
  pub fn bind_type<T>(&self, serializer: SerializerId) -> Result<(), BindingError>
  where
    T: TypeKey, {
    self.bind_key(<T as TypeKey>::type_key(), serializer)
  }

  /// Binds `T` using its [`TypeKey::default_serializer`] when available.
  pub fn bind_type_with_default<T>(&self) -> Result<(), BindingError>
  where
    T: TypeKey, {
    if let Some(serializer) = T::default_serializer() {
      self.bind_type::<T>(serializer)
    } else {
      Ok(())
    }
  }
}

impl Extension for SerializerRegistryExtension {
  fn extension_id(&self) -> ExtensionId {
    self.id
  }

  fn as_any(&self) -> &dyn Any {
    self
  }
}
