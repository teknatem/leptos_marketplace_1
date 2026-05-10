use std::sync::atomic::{AtomicU64, Ordering};

/// Лёгковесный счётчик изменений для одного домена.
/// Инкрементируется при любом изменении, клиент сравнивает с запомненным значением.
pub struct ChangeToken(AtomicU64);

impl ChangeToken {
    pub const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    pub fn bump(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}
