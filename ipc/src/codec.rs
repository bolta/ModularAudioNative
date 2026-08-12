use bson::error::Error;

use crate::Message;

/// M から BSON のバイト列にエンコードする
pub fn encode<M: Message>(message: &M) -> Vec<u8> {
	let mut b = bson::serialize_to_document(message).unwrap();
	b.insert("type", M::TYPE_TAG);
	b.to_vec().unwrap()
}

/// バイト列おから BSON ドキュメントを復元する
// BSON の変換を行うだけの便利関数
// bson::Document を露出させることになるが、
// そうしない（つまり to_bson の処理を decode に含めてしまう）と、
// メッセージの種類の数だけ from_reader が無駄に走ることになるため、この形とする
pub fn to_bson(bytes: &[u8]) -> Result<bson::Document, Error> {
	bson::Document::from_reader(bytes)
}

/// BSON ドキュメント（あらかじめバイト列から to_bson で復元しておく）から M へのデコードを試みる
pub fn decode<M: Message>(doc: &bson::Document) -> Option<M> {
	match doc.get_str("type") {
		Ok(tipe) => if tipe == M::TYPE_TAG {
			bson::deserialize_from_document(doc.clone()).ok()
		} else {
			None
		},
		Err(_) => None,
	}
}
