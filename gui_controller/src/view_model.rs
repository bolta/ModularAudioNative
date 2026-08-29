use std::collections::BTreeMap;

use dioxus::prelude::*;
use ipc::{DomainHint, RegisterSettings, RegisterSettingsItem};

// UI で使うための RegisterSettings の Store 対応版。
// 階層をなす構造体が Store を導出している以外は基本的に同じ

type Items = BTreeMap<String, RegisterSettingsItem>;
pub type RegisterSettingsItemsStore = BTreeMap<String, RegisterSettingsItemStore>;

#[derive(Debug, Store)]
pub struct RegisterSettingsStore {
	pub items: RegisterSettingsItemsStore,
}
impl RegisterSettingsStore {
	pub fn from(orig: RegisterSettings) -> Self {
		Self { items: items_store_from(orig.items) }
	}
}
fn items_store_from(items: Items) -> RegisterSettingsItemsStore {
	items.into_iter().map(|(key, item)| (key, RegisterSettingsItemStore::from(item))).collect()
}

#[derive(Debug, Store)]
pub enum RegisterSettingsItemStore {
	Settings {
		key: String,
		initial: f32,
		original: f32,
		domain: Option<DomainHint>,
	},
	Group(BTreeMap<String, RegisterSettingsItemStore>)
}
impl RegisterSettingsItemStore {
	fn from(orig: RegisterSettingsItem) -> Self {
		match orig {
			RegisterSettingsItem::Settings { key, initial, original, domain } => {
				RegisterSettingsItemStore::Settings {
					key, initial, original,
					domain,
				}
			}
			RegisterSettingsItem::Group(items) => RegisterSettingsItemStore::Group(items_store_from(items)),
		}
	}
}
