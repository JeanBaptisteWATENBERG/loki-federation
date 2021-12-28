#![feature(async_closure)]

use actix_web::{web, App, HttpResponse, HttpServer, Responder, middleware};
use clap::Parser;
use std::path::PathBuf;
use log::{error, info};
use loki_federation_core::{Config, Direction, FederatedLoki};
use serde::{Deserialize, Serialize};
use display_json::{DisplayAsJson};

#[derive(Serialize, Deserialize, Debug, DisplayAsJson)]
struct Query {
    query: String,
    limit: Option<i32>,
    time: Option<i64>,
    direction: Option<Direction>,
}

#[derive(Serialize, Deserialize, Debug, DisplayAsJson)]
struct QueryRange {
    query: String,
    start: i64,
    end: i64,
    limit: Option<i32>,
    direction: Option<Direction>,
    step: Option<String>,
    interval: Option<String>
}

#[derive(Serialize, Deserialize, Debug, DisplayAsJson)]
struct Labels {
    start: Option<i64>,
    end: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, DisplayAsJson)]
struct Series {
    matches: Option<Vec<String>>,
    start: Option<i64>,
    end: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, DisplayAsJson)]
struct LabelPath {
    label: String
}

#[derive(Parser, Debug)]
#[clap(about, version, author)]
struct Args {
    // Path to the config.toml file
    #[clap(short, long, parse(from_os_str), value_name = "FILE")]
    config: PathBuf,
}


struct AppState {
    federated_loki: FederatedLoki,
}


async fn query(data: web::Data<AppState>, query: web::Query<Query>) -> impl Responder {
    info!("Starting to handle query request with params: {}", query.0);
    let query_result = data.federated_loki.query(query.query.to_string(), query.limit, query.time, query.direction).await;
    match query_result {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => {
            error!("An error occured while responding to query request: {}", err);
            HttpResponse::InternalServerError().json("{\"error\": \"Internal Server Error\"}") },
    }
}
async fn query_range(data: web::Data<AppState>, query: web::Query<QueryRange>) -> impl Responder {
    info!("Starting to handle query_range request with params: {}", query.0);
    let query_result = data.federated_loki.query_range(query.query.to_string(), query.start, query.end, query.limit, query.direction, query.step.clone(), query.interval.clone()).await;
    match query_result {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => {
            error!("An error occured while responding to query_range request: {}", err);
            HttpResponse::InternalServerError().json("{\"error\": \"Internal Server Error\"}") },
    }
}

async fn labels(data: web::Data<AppState>, query: web::Query<Labels>) -> impl Responder {
    info!("Starting to handle labels request with params: {}", query.0);
    let result = data.federated_loki.labels(query.start, query.end).await;
    match result {
        Ok(labels) => HttpResponse::Ok().json(labels),
        Err(err) => {
            error!("An error occured while responding to labels request: {}", err);
            HttpResponse::InternalServerError().json("{\"error\": \"Internal Server Error\"}") },
    }
}

async fn label_values(path: web::Path<LabelPath>, data: web::Data<AppState>, query: web::Query<Labels>) -> impl Responder {
    info!("Starting to handle label_values({}) request with params: {}", path.label.to_string(), query.0);
    let result = data.federated_loki.label_values(path.label.to_string(), query.start, query.end).await;
    match result {
        Ok(labels) => HttpResponse::Ok().json(labels),
        Err(err) => {
            error!("An error occured while responding to label_values request: {}", err);
            HttpResponse::InternalServerError().json("{\"error\": \"Internal Server Error\"}") },
    }
}

async fn retrieve_series_get_handler(data: web::Data<AppState>, query: web::Query<Series>) -> impl Responder {
    info!("Starting to handle retrieve_series_get_handler request with params: {}", query.0);
    let result = data.federated_loki.series(query.matches.clone(), query.start, query.end).await;
    match result {
        Ok(series) => HttpResponse::Ok().json(series),
        Err(err) => {
            error!("An error occured while responding to retrieve_series_get_handler request: {}", err);
            HttpResponse::InternalServerError().json("{\"error\": \"Internal Server Error\"}") },
    }
}

async fn retrieve_series_post_handler(data: web::Data<AppState>, query: web::Form<Series>) -> impl Responder {
    info!("Starting to handle retrieve_series_post_handler request with params: {}", query.0);
    let result = data.federated_loki.series(query.matches.clone(), query.start, query.end).await;
    match result {
        Ok(series) => HttpResponse::Ok().json(series),
        Err(err) => {
            error!("An error occured while responding to retrieve_series_post_handler request: {}", err);
            HttpResponse::InternalServerError().json("{\"error\": \"Internal Server Error\"}") },
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let content = std::fs::read_to_string(&args.config)
        .expect("could not read config file");

    let config = toml::from_str::<Config>(&content)
        .expect("could not parse config file");

    env_logger::init_from_env(env_logger::Env::new().default_filter_or(config.debug.log_level));

    let server_bind_address = format!("{}:{}", config.server.bind_address, config.server.port);
    info!("Starting loki-federation on {}", server_bind_address);

    let mut federated_loki = loki_federation_core::FederatedLoki::new(vec![]);
    match config.datasources.name.as_str() {
        "static-http" => {
            let urls = config.datasources.urls.expect("static-http requires urls");
            info!("Using static urls {}", urls.join(", "));
            federated_loki = loki_federation_core::FederatedLoki::new(urls);
        }
        _ => {
            panic!("Unsupported datasource {}", config.datasources.name);
        }
    }

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .data(AppState {
                federated_loki: federated_loki.clone(),
            })
            .route("/ready", web::get().to(|| HttpResponse::Ok().body("ready")))
            .route("/loki/api/v1/query", web::get().to(query))
            .route("/loki/api/v1/query_range", web::get().to(query_range))
            .route("/loki/api/v1/labels", web::get().to(labels))
            .route("/loki/api/v1/label", web::get().to(labels))
            .route("/loki/api/v1/label/{label}/values", web::get().to(label_values))
            .route("/loki/api/v1/series", web::get().to(retrieve_series_get_handler))
            .route("/loki/api/v1/series", web::post().to(retrieve_series_post_handler))
    })
        .bind(server_bind_address)?
        .run()
        .await

}
