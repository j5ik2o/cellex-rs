mod serializer_extension;
#[cfg(test)]
mod tests;

pub use serializer_extension::*;

use alloc::vec::Vec;
use cellex_utils_core_rs::sync::{ArcShared, SharedBound};
use core::any::Any;
use core::fmt::{self, Debug, Formatter};
use portable_atomic::{AtomicI32, Ordering};
use spin::RwLock;

/// 一意な Extension を識別するための ID 型。
pub type ExtensionId = i32;

static NEXT_EXTENSION_ID: AtomicI32 = AtomicI32::new(0);

/// 新しい ExtensionId を払い出します。
#[must_use]
pub fn next_extension_id() -> ExtensionId {
  NEXT_EXTENSION_ID.fetch_add(1, Ordering::SeqCst)
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
    let handle = extension.into_dyn(|ext| ext as &dyn Extension);
    let idx = id as usize;
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
