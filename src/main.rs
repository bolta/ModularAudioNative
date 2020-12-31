//use core::node_host;
mod core;

use crate::core::node::*;
use crate::core::node_host::*;
use crate::core::signal::*;
use typed_arena::Arena;

fn main() {
	let mut host = NodeHost::new();
	let mut var = 0f32;
	let gen = host.add_node(Node::from_closure(Box::new(move || {
		let result = var;
		var += 0.01f32;
		Some(result)
	})));
	let neg = host.add_node(Node::from_closure(Box::new(move || {
		//_ gen は host の管理下にあるので、このクロージャで所有してはいけないが
		// （だからクロージャに move をつけてはいけないのは間違いないだろう）、
		// move をつけないと、このクロージャがある間 gen も必ずあることを伝えられない。
		// （実際 gen と neg はともに host（の Arena）の管理下にあるので
		// 寿命は同一である）
		// 他の伝える方法があるのだろうか？？
		// → ということで、参照を自由に受け渡せるように弱参照 Weak を導入し（これなら move できる）、
		// Weak は Rc から得るものなので Rc を導入し、
		// Rc で直接管理するオブジェクトは変更できないので、ごまかすために RefCell を導入した。
		// Rc は NodeHost の外には出てこないが、Weak と RefCell は使用側ではがす操作が必要になって
		// すごく使い勝手が悪い。
		// やりたいことは "- gen" だけなので、そう書きたいのだが…
		// NodeHandle みたいなものを導入して、
		// Node そのもののように扱えるようなメソッドや演算子を提供しつつ
		// 裏で参照操作を隠蔽するみたいな形にすべきか…
		// NodeHandle は、Weak をラップし、演算子などの適用に対して Host に新しい Node を生成して
		// その handle を返す（そのためには NodeHost の参照を持っておく必要がある）。
		// あるいは、いっそ NodeHost::nodes の添え字を ID として持つことにすれば、
		// Rc とか Weak は使う必要がなくなる？？
		Some(- gen.upgrade().unwrap().borrow().current())
	})));

	host.play();
}
