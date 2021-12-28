#![feature(async_closure)]

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use clap::Parser;
use std::path::PathBuf;
use loki_federation_core::{Config, Direction, FederatedLoki};
use serde::Deserialize;

#[derive(Deserialize)]
struct Query {
    query: String,
    limit: Option<i32>,
    time: Option<i64>,
    direction: Option<Direction>,
}

#[derive(Deserialize)]
struct QueryRange {
    query: String,
    start: i64,
    end: i64,
    limit: Option<i32>,
    direction: Option<Direction>,
    step: Option<String>,
    interval: Option<String>
}

#[derive(Deserialize)]
struct Labels {
    start: Option<i64>,
    end: Option<i64>,
}

#[derive(Deserialize)]
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
    let query_result = data.federated_loki.query(query.query.to_string(), query.limit, query.time, query.direction).await;
    match query_result {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => HttpResponse::InternalServerError().json("{\"error\": \"Internal Server Error\"}"),
    }
}
async fn query_range(data: web::Data<AppState>, query: web::Query<QueryRange>) -> impl Responder {
    let query_result = data.federated_loki.query_range(query.query.to_string(), query.start, query.end, query.limit, query.direction, query.step.clone(), query.interval.clone()).await;
    match query_result {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(err) => HttpResponse::InternalServerError().json("{\"error\": \"Internal Server Error\"}"),
    }
}

async fn labels(data: web::Data<AppState>, query: web::Query<Labels>) -> impl Responder {
    let result = data.federated_loki.labels(query.start, query.end).await;
    match result {
        Ok(labels) => HttpResponse::Ok().json(labels),
        Err(err) => HttpResponse::InternalServerError().json("{\"error\": \"Internal Server Error\"}"),
    }
}

async fn label_values(path: web::Path<LabelPath>, data: web::Data<AppState>, query: web::Query<Labels>) -> impl Responder {
    let result = data.federated_loki.label_values(path.label.to_string(), query.start, query.end).await;
    match result {
        Ok(labels) => HttpResponse::Ok().json(labels),
        Err(err) => HttpResponse::InternalServerError().json("{\"error\": \"Internal Server Error\"}"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    // // let query = "{service=\"test\"}".to_string();
    // // let time: i64 = 1639946473000000000;
    //
    // let query = "{service=\"test\"}".to_string();
    // let response = federatedLoki.query_range(query, 1639940400000000000, 1639947600000000000, Some(1000), None, Some("5".to_string()), None).await?;

    let args = Args::parse();

    let content = std::fs::read_to_string(&args.config)
        .expect("could not read config file");

    let config = toml::from_str::<Config>(&content)
        .expect("could not parse config file");


    let server_bind_address = format!("{}:{}", config.server.bind_address, config.server.port);
    println!("Starting loki-federation on {}", server_bind_address);

    let mut federated_loki = loki_federation_core::FederatedLoki::new(vec![]);
    match config.datasources.name.as_str() {
        "static-http" => {
            let urls = config.datasources.urls.expect("static-http requires urls");
            println!("Using static urls {}", urls.join(", "));
            federated_loki = loki_federation_core::FederatedLoki::new(urls);
        }
        _ => {
            panic!("Unsupported datasource {}", config.datasources.name);
        }
    }

    HttpServer::new(move || {
        App::new()
            .data(AppState {
                federated_loki: federated_loki.clone(),
            })
            .route("/ready", web::get().to(|| HttpResponse::Ok().body("ready")))
            .route("/loki/api/v1/query", web::get().to(query))
            .route("/loki/api/v1/query_range", web::get().to(query_range))
            .route("/loki/api/v1/labels", web::get().to(labels))
            .route("/loki/api/v1/label", web::get().to(labels))
            .route("/loki/api/v1/label/{label}/values", web::get().to(label_values))
    })
        .bind(server_bind_address)?
        .run()
        .await

}
