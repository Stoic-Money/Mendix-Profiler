
use std::{
    net::TcpListener,
    thread,
};

mod profiler_client;
mod microflow_execution;
mod profile_session;
use log::{error, info, LevelFilter};
use env_logger::Builder;
pub use profile_session::ProfileSession;
pub use profiler_client::ProfilerClient;



fn main() -> std::io::Result<()> {
    // let mut builder = Builder::from_default_env();
    // builder.target(Target::Stdout);
    // builder.init();
    // let env = Env::default()
    //     .filter_or("MY_LOG_LEVEL", "trace")
    //     .write_style_or("MY_LOG_STYLE", "always");
    Builder::new().filter_level(LevelFilter::Info).init();

    // env_logger::init_from_env(env);

    let port = 12345;
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(addr)?;
    info!("Server running on localhost:{}", port);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    let mut client = ProfilerClient::new(stream);
                    client.handle_connection();
                });
            }
            Err(e) => {
                error!("Failed to accept connection; err = {:?}", e);
            }
        }
    }

    Ok(())
}
