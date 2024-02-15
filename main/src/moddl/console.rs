use std::fmt::Display;

pub fn warn<T>(message: T)
where T: Display
{
	println!("[warning] {}", message);
}
