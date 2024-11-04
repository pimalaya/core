use std::time::Duration;

#[cfg(feature = "async-std")]
pub async fn async_std_sleep(duration: Duration) {
    async_std::task::sleep(duration).await
}

#[cfg(feature = "std")]
pub fn std_sleep(duration: Duration) {
    std::thread::sleep(duration)
}

#[cfg(feature = "tokio")]
pub async fn tokio_sleep(duration: Duration) {
    tokio::time::sleep(duration).await
}
