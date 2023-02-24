use actix_web::get;
use actix_web::HttpResponse;

#[get("/metrics")]
pub async fn prometheus() -> HttpResponse {
    use ::prometheus::Encoder;
    let encoder = ::prometheus::TextEncoder::new();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&::prometheus::default_registry().gather(), &mut buffer) {
        log::warn!("could not encode custom metrics: {}", e);
    };
    let res = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("custom metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };
    buffer.clear();
    HttpResponse::Ok().body(res)
}
