#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
/// MML でサポートする機能のうち、ノード構造等「外側」で対応が必要なもの
pub enum Feature {
	Volume,
	Velocity,
	Detune,
}
