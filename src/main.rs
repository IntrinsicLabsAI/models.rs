use model_server::GgmlServer;
use model_server::Server;
use model_server::GgmlContext;

use anyhow::Result;


#[tokio::main]
async fn main() -> Result<()> {
    // Make a context and then we can share it around
    let ctx = GgmlContext::new(16 * 1024 * 1024);

    let srv = GgmlServer {
        ctx: ctx,
    };

    srv.serve("127.0.0.1:8000").await?;

    Ok(())
}
