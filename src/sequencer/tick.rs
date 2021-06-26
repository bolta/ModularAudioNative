use std::cell::RefCell;
use std::rc::Rc;

use super::super::core::node_host::NodeHost;
use super::super::core::sample_user::SampleUser;

pub struct Tick {
	sample_rate: i32,
	users: Vec<Rc<RefCell<dyn TickUser>>>,
	tempo: f32,
	ticks_per_beat: i32,
	/**
	 * タイマ。1 以上で sample() が呼ばれると Tick が発行され、タイマは 1 で割った余りまで減る。
	 * 最初のサンプルで最初の Tick を発行するよう 1 から始める
	 */
	timer: f32,
}

impl Tick {
	pub fn new(host: &mut NodeHost, tempo: f32, ticks_per_beat: i32) -> Rc<RefCell<Tick>> {
		let result = Rc::new(RefCell::new(Tick {
			sample_rate: host.sample_rate(),
			users: vec![],
			tempo,
			ticks_per_beat,
			timer: 1f32,
		}));

		host.add_sample_user(result.clone());

		result
	}

	pub fn add_user(&mut self, user: Rc<RefCell<dyn TickUser>>) {
		self.users.push(user);
	}
}

impl SampleUser for Tick {
	fn sample(&mut self) {
		while self.timer >= 1f32 {
			for u in &mut self.users {
				u.borrow_mut().tick();
			}
			self.timer -= 1f32;
		}
		self.timer += self.tempo * self.ticks_per_beat as f32 / 60f32 / self.sample_rate as f32;
	}
}

pub trait TickUser {
	fn tick(&mut self);
}
