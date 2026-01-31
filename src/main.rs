mod messages;
mod sockets;

use tracing::level_filters::LevelFilter;

use tracing_subscriber::{Layer as _, layer::SubscriberExt as _, util::SubscriberInitExt as _};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let terminal_layer = tracing_subscriber::fmt::layer()
        .compact()
        // TODO: allow configuring the displayed log level with
        // either cli arguments or environment variables
        .with_filter(if cfg!(debug_assertions) {
            LevelFilter::TRACE
        } else {
            LevelFilter::INFO
        })
        .boxed();

    tracing_subscriber::registry().with(terminal_layer).init();

    tokio::spawn(async {
        let _ = sockets::testing().await;
    });

    tokio::signal::ctrl_c().await?;

    Ok(())
}
