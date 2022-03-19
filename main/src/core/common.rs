pub type Sample = f32;
pub type SampleCount = i32;

/// NodeHost における Node の添字。
/// 単なる添字なので出力チャンネル数の情報は持たない
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeIndex(pub usize);

// 以下は NodeIndex にチャンネル数の情報を加えたもの。
// それぞれの型には互換性がない。
// channeled メソッドで後述の ChanneledNodeIndex へ「アップキャスト」することができる

/// 出力のないノードを指すことが保証された NodeIndex
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NoOutputNodeIndex(NodeIndex);
impl NoOutputNodeIndex {
	pub fn channeled(&self) -> ChanneledNodeIndex { ChanneledNodeIndex::NoOutput(*self) }
}

/// モノラルのノードを指すことが保証された NodeIndex
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MonoNodeIndex(NodeIndex);
impl MonoNodeIndex {
	pub fn channeled(&self) -> ChanneledNodeIndex { ChanneledNodeIndex::Mono(*self) }
}

/// ステレオのノードを指すことが保証された NodeIndex
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StereoNodeIndex(NodeIndex);
impl StereoNodeIndex {
	pub fn channeled(&self) -> ChanneledNodeIndex { ChanneledNodeIndex::Stereo(*self) }
}

/// チャンネルつき NodeIndex をまとめて扱うための enum。
/// as_~~ メソッドで具体的なチャンネル数の **NodeIndex へ「ダウンキャスト」することができる。
/// また、unchanneled メソッドで生の NodeIndex を取り出すことができる
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ChanneledNodeIndex {
	NoOutput(NoOutputNodeIndex),
	Mono(MonoNodeIndex),
	Stereo(StereoNodeIndex),
}
impl ChanneledNodeIndex {
	pub fn no_output(index: usize) -> Self { Self::NoOutput(NoOutputNodeIndex(NodeIndex(index))) }
	pub fn mono(index: usize) -> Self { Self::Mono(MonoNodeIndex(NodeIndex(index))) }
	pub fn stereo(index: usize) -> Self { Self::Stereo(StereoNodeIndex(NodeIndex(index))) }

	pub fn unchanneled(&self) -> NodeIndex {
		match self {
			Self::NoOutput(NoOutputNodeIndex(index)) => *index,
			Self::Mono(MonoNodeIndex(index)) => *index,
			Self::Stereo(StereoNodeIndex(index)) => *index,
		}
	}
	pub fn as_mono(&self) -> MonoNodeIndex {
		match self {
			Self::Mono(mono) => *mono,
			_ => panic!("not mono"),
		}
	}
	pub fn as_stereo(&self) -> StereoNodeIndex {
		match self {
			Self::Stereo(stereo) => *stereo,
			_ => panic!("not stereo"),
		}
	}
	pub fn channels(&self) -> i32 {
		match self {
			Self::NoOutput(_) => 0,
			Self::Mono(_) => 1,
			Self::Stereo(_) => 2,
		}
	}
}

pub const PI: f32 = std::f32::consts::PI;
pub const TWO_PI: f32 = 2_f32 * PI;

pub const NO_OUTPUT: Sample = f32::NAN;
