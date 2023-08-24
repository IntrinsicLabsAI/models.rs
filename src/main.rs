use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::Arc;

use parsers::ggml;

use anyhow::Result;
// use futures::{lock::Mutex, FutureExt, TryFutureExt};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::{self, net::TcpStream};

struct Context {
    ptr: *mut ggml::ggml_context,
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

async fn handle_req(
    mut client: TcpStream,
    addr: SocketAddr,
    ctx: Arc<Mutex<Context>>,
) -> Result<()> {
    println!("New connection from {}", addr);
    // The guard will kill the lock when it goes out of scope
    let value = unsafe {
        let guard = ctx.deref().lock().await;
        let tensor =
            ggml::ggml_new_tensor_1d(guard.ptr, ggml::ggml_type_GGML_TYPE_F32, 10);
        for i in 0..10 {
            ggml::ggml_set_f32_1d(tensor, i, (-i as f32) / 100.0);
        }
        let gelu = ggml::ggml_gelu(guard.ptr, tensor);

        // Compute the tensor
        let cf = ggml::ggml_build_forward_ctx(guard.ptr, gelu);
        ggml::ggml_graph_compute_with_ctx(guard.ptr, cf, 4);

        let mut items = Vec::new();
        for i in 0..10 {
            items.push(ggml::ggml_get_f32_1d(gelu, i));
        }

        // Get access to a global mutex for creating a new instance.
        // We can have up to some number of instances that access all of the globals here.
        // Have a queue that automaticaly on deref re-accepts the object, performing any necessary reset operations to
        // get the object back into its former state

        items
    };

    client
        .write(format!("{:?}\n", value).as_bytes())
        .await
        .unwrap();
    // Get the value of this thing

    let has_neon = unsafe { ggml::ggml_cpu_has_neon() };
    let data = format!("Has NEON? {}", has_neon);

    client.write(data.as_bytes()).await.unwrap();
    client.shutdown().await.unwrap();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Want the server to run in its own spawned component. Nested spawns?
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;

    // Make a context and then we can share it around
    let ctx = unsafe {
        ggml::ggml_init(ggml::ggml_init_params {
            mem_size: 16 * 1024 * 1024,
            mem_buffer: std::ptr::null_mut(),
            no_alloc: false,
        })
    };

    let shared_ctx = Arc::new(Mutex::new(Context { ptr: ctx }));

    loop {
        let (client, addr) = listener.accept().await.unwrap();
        tokio::spawn(handle_req(client, addr, shared_ctx.clone()));
    }
}
