use super::{InitDbResult, InitializeDb};
use futures::lock::{MappedMutexGuard as AsyncMappedMutexGuard, Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard};

pub type DbLocked<'a, Db> = AsyncMappedMutexGuard<'a, Option<Db>, Db>;

pub async fn get_or_initialize_db<Db>(mutex: &AsyncMutex<Option<Db>>) -> InitDbResult<DbLocked<'_, Db>>
where
    Db: InitializeDb,
{
    let mut locked_db = mutex.lock().await;
    // Db is initialized already
    if locked_db.is_some() {
        return Ok(unwrap_tx_history_db(locked_db));
    }

    let db = Db::init().await?;
    *locked_db = Some(db);
    Ok(unwrap_tx_history_db(locked_db))
}

/// # Panics
///
/// This function will `panic!()` if the inner value of the `guard` is `None`.
fn unwrap_tx_history_db<Db>(guard: AsyncMutexGuard<'_, Option<Db>>) -> DbLocked<'_, Db> {
    AsyncMutexGuard::map(guard, |wrapped_db| {
        wrapped_db
            .as_mut()
            .expect("The locked 'Option<Db>' must contain a value")
    })
}
