pub fn get_random_element<T>(array: &[T]) -> anyhow::Result<&T> {
    let index = fastrand::usize(..array.len());
    Ok(&array[index])
}
