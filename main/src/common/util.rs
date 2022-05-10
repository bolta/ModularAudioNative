pub fn ignore_errors<T, E>(result: Result<T, E>) {
	result.ok();
}
