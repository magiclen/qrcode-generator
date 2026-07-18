//! Renders a QR Code and writes it to files through tokio's asynchronous file API.
//!
//! Rendering itself is synchronous; only writing and flushing are asynchronous.
//!
//! Run with: `cargo run --example tokio_file --features tokio`

use qrcode_generator::{
    Renderer,
    qr::{Encoder, ErrorCorrection},
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let symbol = Encoder::new(ErrorCorrection::Medium).encode_text("Hello, tokio!").unwrap();

    // Write to a file we open ourselves; any tokio::io::AsyncWrite works here.
    let mut file = tokio::fs::File::create("tokio_writer_output.svg").await.unwrap();
    Renderer::new(&symbol, 512).write_svg_async(&mut file, None::<&str>).await.unwrap();

    // Or save to a path atomically, like the synchronous save_svg.
    Renderer::new(&symbol, 512)
        .save_svg_async("tokio_save_output.svg", None::<&str>)
        .await
        .unwrap();

    println!("wrote tokio_writer_output.svg and tokio_save_output.svg");
}
