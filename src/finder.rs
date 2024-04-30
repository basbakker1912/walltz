pub fn find_best_by_value<U, T, C, F>(
    value: U,
    iter: T,
    get_value: C,
    equality_check: F,
) -> (bool, Option<T::Item>)
where
    U: PartialEq,
    T: Iterator,
    C: Fn(&T::Item) -> U,
    F: Fn(&U, &U) -> u32,
{
    let optimal_value = iter
        .map(|rhs| {
            let equality = equality_check(&value, &get_value(&rhs));

            (rhs, equality)
        })
        .max_by(|(_, v1), (_, v2)| v1.cmp(v2));

    match optimal_value {
        Some((optimal_value, _)) => (value == get_value(&optimal_value), Some(optimal_value)),
        None => (false, None),
    }
}

pub fn check_string_equality(v1: &str, v2: &str) -> u32 {
    v1.chars()
        .into_iter()
        .zip(v2.chars().into_iter())
        .fold(
            0u32,
            |output, (v1, v2)| {
                if v1 == v2 {
                    output + 1
                } else {
                    output
                }
            },
        )
}
