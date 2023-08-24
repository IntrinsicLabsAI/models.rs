use std::ptr::NonNull;
use std::sync::Arc;

use anyhow::Result;
use tokio::io::AsyncWriteExt;
use tokio::net::{ToSocketAddrs, TcpStream};

#[macro_use]
extern crate async_trait;

#[allow(
    dead_code,
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case
)]
pub mod ggml;

#[derive(Clone, Copy)]
pub struct GgmlContext {
    pub ptr: NonNull<ggml::ggml_context>,
}

impl GgmlContext {
    pub fn new(mem_size: usize) -> Self {
        let ctx = unsafe {
            ggml::ggml_init(ggml::ggml_init_params {
                mem_size: mem_size,
                mem_buffer: std::ptr::null_mut(),
                no_alloc: false,
            })
        };

        GgmlContext {
            ptr: unsafe { NonNull::new_unchecked(ctx) },
        }
    }
}

unsafe impl Send for GgmlContext {}
unsafe impl Sync for GgmlContext {}

#[async_trait]
pub trait Handler<C>
where
    C: Send + Sync,
{
    async fn handle(&self, client: TcpStream, ctx: C);
}

#[async_trait]
pub trait Server<C, H>
where C: Send + Sync
{
    type HandlerType: Handler<Arc<C>>;

    /// Make a new Handler<C> instance that can accept the proper context type.
    fn make_handler(&self) -> Self::HandlerType;

    async fn serve<A: ToSocketAddrs + Send>(&self, addr: A) -> Result<()>;
}

pub struct GgmlHandler;

#[async_trait::async_trait]
impl Handler<Arc<GgmlContext>> for GgmlHandler {
    async fn handle(&self, mut client: TcpStream, _ctx: Arc<GgmlContext>) {
        client.write(b"executing...").await.unwrap();
        client.shutdown().await.unwrap();
    }
}

pub struct GgmlServer{
    pub ctx: GgmlContext,
}

#[async_trait]
impl Server<GgmlContext, GgmlHandler> for GgmlServer {
    type HandlerType = GgmlHandler;

    fn make_handler(&self) -> GgmlHandler {
        GgmlHandler
    }

    async fn serve<A: ToSocketAddrs + Send>(&self, addr:A) -> Result<()> {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let ctx = Arc::new(self.ctx);
        loop {
            let handler = self.make_handler();
            let (client, addr) = listener.accept().await.unwrap();
            println!("Handling request from {}", addr);
            let ctx = Arc::clone(&ctx);
            tokio::spawn(async move {
                handler.handle(client, ctx).await;
            });
        }
    }
}
