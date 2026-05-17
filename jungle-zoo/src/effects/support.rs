pub(crate) async fn maybe_delay() {
    #[cfg(feature = "delay")]
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
}
