#[expect(clippy::disallowed_types)]
pub type HashMap<K, V> = std::collections::HashMap<K, V>;

#[expect(clippy::disallowed_types)]
pub type HashSet<T> = std::collections::HashSet<T>;
