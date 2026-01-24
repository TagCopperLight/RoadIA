mod api;
mod map;
mod simulation;

use axum::serve;
use axum::{extract::ws::WebSocketUpgrade, extract::State, routing::get, Json, Router};
use tokio::net::TcpListener;

use tower_http::cors::{Any, CorsLayer};

use crate::api::server::websocket_loop;
use crate::map::intersection::{Intersection, IntersectionKind};
use crate::map::model::Map;
use crate::map::road::{Road, RoadType};
use crate::simulation::config::SimulationConfig;
use crate::simulation::handle::Handle;
use crate::simulation::vehicle::{
    TripRequest, Vehicle, VehicleKind, VehicleSpec,
};

fn main(){

}