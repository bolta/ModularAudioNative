use crate::core::node::*;

/// 安全なはずだが寿命チェックに引っかかってしまう場合、
/// 一旦ポインタにすることでチェックを逃れる（こんな方法しかないのか？？）
pub fn discard_lifetime<'a>(node: &'a Node) -> &'static Node {
	unsafe { &* (node as *const Node) }
}
