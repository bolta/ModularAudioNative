use std::cell::RefCell;
use std::rc::Rc;

use super::notable::*;
use super::sequence_thread::*;
use super::super::node::var::*;

pub enum Instruction<'a> {
	// シリアライゼーションなどを考えたときに参照ではなく文字列を保持した方がよさそうに思う
	// Value { target: /* String */ Rc<RefCell<VarController>>, value: f32 },
	Value { target: /* String */ &'a mut VarController, value: f32 },
	// シリアライゼーションなどを考えたときに参照ではなく文字列を保持した方がよさそうに思う
	// Note { target: /* String */ Rc<RefCell<dyn Notable>>, note_on: bool },
	Note { target: /* String */ &'a mut Notable, note_on: bool },
	Wait { wait: i32 },
	// Parameter,
	// Jump,
}

impl<'a> Instruction<'a> {
	pub fn execute(&self, thread: &mut SequenceThread) {
		match self {
			Self::Value { target, value } => {
				// target.borrow_mut().set(*value);
				target.set(*value);
			}


			Self::Note { target, note_on } => {
				if *note_on {
					// target.borrow_mut().note_on();
					target.note_on();
				} else {
					// target.borrow_mut().note_off();
					target.note_off();
				}
			}

			Self::Wait { wait } => {
				assert_eq!(thread.wait(), 0);
				thread.set_wait(*wait);
			}
		}
	}
}

