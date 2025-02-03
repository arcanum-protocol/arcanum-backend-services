#[sqlx::test]
async fn happy_path(pool: sqlx::SqlitePool) -> anyhow::Result<()> {
    Indexer::builder()
        .sqlite_storage(pool)
        .http_rpc_url("https://")
        .ws_rpc_url("https://")
        .fetch_interval(Duration::from_millis(100))
        .filter(Filter::new().events([
            multipool_types::Multipool::TargetShareChange::SIGNATURE,
            multipool_types::Multipool::AssetChange::SIGNATURE,
            multipool_types::Multipool::FeesChange::SIGNATURE,
        ]))
        .set_processor(EmbededProcessor)
}
    Ok(())
}
