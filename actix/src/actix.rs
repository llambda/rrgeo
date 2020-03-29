#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
use actix_web::{error, http, middleware, web, App, HttpServer, HttpResponse, Result};

use reverse_geocoder::{
    Locations,
    Record,
    ReverseGeocoder,
    ReverseGeocodeError,
};
use std::fmt;

#[derive(Debug)]
enum MyError {
    NotFound,
    InternalError,
}


impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MyError::NotFound => write!(f, "Not found"),
            MyError::InternalError => write!(f, "Internal error"),
        }
    }
}

impl error::ResponseError for MyError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            MyError::NotFound => HttpResponse::new(http::StatusCode::NOT_FOUND),
            MyError::InternalError => HttpResponse::new(http::StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}

#[derive(Deserialize)]
struct LatLong {
    lat: f64,
    long: f64,
}

async fn index(lat_long: web::Query<LatLong>) -> Result<web::Json<Record>, MyError> {
    lazy_static! {
        static ref LOCATIONS: Locations = Locations::from_memory();
        static ref GEOCODER: ReverseGeocoder<'static> = ReverseGeocoder::new(&LOCATIONS);
    }

    let res = match GEOCODER.search(&[lat_long.lat, lat_long.long]) {
        Ok(result) => result,
        Err(error) => match error {
            ReverseGeocodeError::NoResultsFound => return Err(MyError::NotFound),
            ReverseGeocodeError::KdTreeError(_error_kind) => return Err(MyError::InternalError),
        }
    };

    Ok(web::Json((*res.1).clone()))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .route("/", web::get().to(index))
    })
    .keep_alive(10)
    .bind("127.0.0.1:3000")?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate bytes;

    use actix_web::dev::Service;
    use actix_web::{http, test, web, App};

    #[actix_rt::test]
    async fn it_serves_results_on_actix() -> Result<(), MyError> {
        let mut app = test::init_service(
            App::new().route("/", web::get().to(index))
        )
        .await;

        let req = test::TestRequest::get().uri("/?lat=44.962786&long=-93.344722").to_request();

        let resp = app.call(req).await.unwrap();

        assert_eq!(resp.status(), http::StatusCode::OK);

        let response_body = match resp.response().body().as_ref() {
            Some(actix_web::body::Body::Bytes(bytes)) => bytes,
            _ => panic!("Response error"),
        };

        assert_eq!(response_body, r##"{"lat":44.9483,"lon":-93.34801,"name":"Saint Louis Park","admin1":"Minnesota","admin2":"Hennepin County","admin3":"US"}"##);

        Ok(())
    }
}
