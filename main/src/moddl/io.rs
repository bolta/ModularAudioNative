use std::{rc::Rc, cell::RefCell, collections::HashMap};

use parser::common::Location;
use rand::rngs::StdRng;
use rand::prelude::*;

use super::{
	value::*, function::{Function, check_arity}, scope::Scope, error::ModdlResult,
};

pub trait Io {
	fn perform(&mut self, loc: &Location) -> ModdlResult<Value>;
}

pub struct Rand {
	gen: StdRng,
}
impl Rand {
	pub fn new() -> Self {
		Self { gen: StdRng::from_entropy() }
	}
}
impl Io for Rand {
	fn perform(&mut self, loc: &Location) -> ModdlResult<Value> {
		Ok((ValueBody::Float(self.gen.gen()), loc.clone()))
	}
}

pub struct ThenIo {
	predecessor: Rc<RefCell<dyn Io>>,
	successor: Rc<dyn Function>,
	vars: Rc<RefCell<Scope>>,
}
impl ThenIo {
	pub fn new(predecessor: Rc<RefCell<dyn Io>>, successor: Rc<dyn Function>, vars: Rc<RefCell<Scope>>) -> Self {
		Self { predecessor, successor, vars }
	}
}
impl Io for ThenIo {
	fn perform(&mut self, loc: &Location) -> ModdlResult<Value> {
		// TODO perform した結果がまた Io の場合、perform を繰り返す必要があるか？
		let pred_value = self.predecessor.borrow_mut().perform(loc) ?;
		let sig = self.successor.signature();
		check_arity(&sig, 1, &/* mapper_loc */loc) ?; // TODO 位置情報が適切でないかも

		self.successor.call(& HashMap::from([(sig[0].clone(), pred_value.clone())]), &self.vars, /* mapper_loc */loc.clone()) // TODO 位置情報が適切でないかも
	}
}
