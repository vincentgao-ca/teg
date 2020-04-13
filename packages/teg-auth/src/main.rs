// #[macro_use] extern crate async_std;
#[macro_use] extern crate juniper;
#[macro_use] extern crate log;

// #[macro_use] extern crate log;
// #[macro_use] extern crate graphql_client;
// extern crate tokio;
extern crate reqwest;
extern crate secp256k1;
extern crate rand;
extern crate rmp_serde as rmps;
// extern crate futures;
// extern crate futures03;
extern crate serde;
// extern crate serde_json;
extern crate url;
extern crate gravatar;

use warp::{http::Response, Filter};
use dotenv::dotenv;
use std::env;
use std::sync::Arc;

pub mod models;
mod context;
mod graphql_schema;

pub use context::Context;
pub use graphql_schema::{ Schema, Query, Mutation };

use async_std::task;

error_chain::error_chain! {}

#[async_std::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    if std::env::args().any(|arg| arg == "migrate") {
        eprintln!("Running Auth Migrations [TODO: Not yet implemented!]");

        // use diesel::prelude::*;
        //
        // let database_url = env::var("POSTGRESQL_ADDON_URI")
        //     .expect("POSTGRESQL_ADDON_URI must be set");
        //
        // let connection = PgConnection::establish(&database_url)
        //     .expect(&format!("Error connecting to {}", database_url));
        //
        // // This will run the necessary migrations.
        // embedded_migrations::run(&connection)
        //     .chain_err(|| "Error running migrations")?;

        // By default the output is thrown out. If you want to redirect it to stdout, you
        // should call embedded_migrations::run_with_output.
        // embedded_migrations::run_with_output(&connection, &mut std::io::stdout())
        //     .chain_err(|| "Error running migrations")?;

        eprintln!("Running Auth Migrations: DONE");

        return Ok(())
    }

    eprintln!("Starting Auth Server");

    let log = warp::log("auth");

    let port = env::var("PORT")
        .expect("$PORT must be set")
        .parse()
        .expect("Invalid $PORT");

    let homepage = warp::path::end().map(|| {
        Response::builder()
            .header("content-type", "text/html")
            .body(format!(
                "<html><h1>juniper_warp</h1><div>visit <a href=\"/graphiql\">/graphiql</a></html>"
            ))
    });

    let database_url = env::var("POSTGRESQL_ADDON_URI")
        .expect("$POSTGRESQL_ADDON_URI must be set");

    let pool = sqlx::PgPool::new(&database_url)
        .await
        .map(|p| Arc::new(p))
        .expect("Could not connect to Postgres");

    models::Invite::generate_or_display_initial_invite(
        Arc::clone(&pool)
    )
        .await
        .map_err(|err| {
            format!("{:?}", err)
        })?;

    let schema = Schema::new(Query, Mutation{});


    // Firebase Certs
    use async_std::sync::RwLock;

    let pem_keys = models::jwt::get_pem_keys()?;
    let pem_keys_lock = Arc::new(RwLock::new(pem_keys));

    let pem_keys_refresh = Arc::clone(&pem_keys_lock);

    use futures::stream::StreamExt;

    let firebase_refresh_task = async_std::stream::repeat(())
        .fold(pem_keys_refresh, |pem_keys_refresh, _| async move {
            info!("Firebase certs will refresh in an hour");
            task::sleep(std::time::Duration::from_secs(60 * 60)).await;

            let next_pem_keys = models::jwt::get_pem_keys().expect("Unable to refresh Firebase certs");

            let pem_keys_borrow = Arc::clone(&pem_keys_refresh);
            let mut writer = pem_keys_borrow.write().await;

            *writer = next_pem_keys;

            pem_keys_refresh
        });

    task::spawn(firebase_refresh_task);

    // Video Streaming

    // // Initialize GStreamer first
    // gst::init()
    //     .map_err(|err| format!("gst::init(): {:?}", err))?;
    //
    // crate::models::video::check_plugins()
    //     .map_err(|err| format!("check_plugins(): {:?}", err))?;
    //
    // // Create our application state
    // let (video, send_gst_msg_rx) = models::video::App::new()
    //     .await
    //     .map_err(|err| format!("Unable to start video provider: {:?}", err))?;
    //
    // let video_clone = video.clone();
    //
    // task::spawn(async move {
    //     let mut send_gst_msg_rx = send_gst_msg_rx.fuse();
    //     loop {
    //         // Pass the GStreamer messages to the application control logic
    //         let gst_msg = send_gst_msg_rx.select_next_some().await;
    //         let _ = video_clone.handle_pipeline_message(&gst_msg)
    //             .expect("Pipeline Error");
    //     }
    // });

    // State
    let state = warp::any()
        .and(warp::header::optional::<i32>("user-id"))
        .and_then(move |user_id| {
            task::block_on(
                Context::new(
                    Arc::clone(&pool),
                    user_id,
                    Arc::clone(&pem_keys_lock),
                    // video.clone(),
                )
            ).map_err(|err| {
                warp::reject::custom(err)
            })
        });

    let graphql_filter = juniper_warp::make_graphql_filter(schema, state.boxed());

    warp::serve(
        warp::get2()
            .and(warp::path("graphiql"))
            .and(juniper_warp::graphiql_filter("/graphql"))
            .or(homepage)
            .or(warp::path("graphql").and(graphql_filter))
            .with(log),
    )
        .run(([127, 0, 0, 1], port));

    Ok(())
}
