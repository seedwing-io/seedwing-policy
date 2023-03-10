use actix_web::{
    get,
    http::header::CONTENT_TYPE,
    web::{self},
    HttpResponse,
};
use okapi::openapi3::{
    Components, ExampleValue, MediaType, OpenApi, Operation, PathItem, Ref, RefOr, RequestBody,
    Response, Responses, SchemaObject,
};
use okapi::schemars::schema::{Metadata, Schema};
use seedwing_policy_engine::runtime::{Example, World};

const APPLICATION_JSON: &str = "application/json";

const RESPONSE_SUCCESS: &str = "validation_success";
const RESPONSE_FAILURE: &str = "validation_failure";

#[get("/openapi.json")]
pub async fn openapi(world: web::Data<World>) -> HttpResponse {
    let mut api = OpenApi::default();

    api.openapi = "3.0.0".into();
    api.info.title = "Seedwing Policy Server".into();
    api.info.version = "0.1".into();

    let mut schemas = okapi::Map::default();

    let default_post_responses = build_default_post_response();

    for (name, pattern) in world.all() {
        if !pattern.parameters().is_empty() {
            continue;
        }

        let path = format!("/api/policy/v1alpha1/{}", name.as_type_str());
        let mut path_item = PathItem::default();

        let mut get = Operation::default();

        get.description = Some("Retrieve the pattern definition".into());

        let mut post = Operation::default();

        post.description = pattern.documentation();
        let mut content = okapi::Map::new();
        let json_schema = pattern.as_json_schema(world.as_ref(), &vec![]);

        if let Schema::Object(mut json_schema) = json_schema {
            json_schema.metadata = Some(Box::new(Metadata {
                id: None,
                title: Some(name.as_type_str().into()),
                description: None,
                default: None,
                deprecated: false,
                read_only: false,
                write_only: false,
                examples: vec![],
            }));

            schemas.insert(name.as_type_str(), json_schema);

            let mut json_schema = SchemaObject::default();
            json_schema.reference = Some(format!("#/components/schemas/{}", name.as_type_str()));

            let json_media_type = MediaType {
                schema: Some(json_schema),
                example: None,
                examples: build_examples(pattern.examples()),
                encoding: Default::default(),
                extensions: Default::default(),
            };

            content.insert(APPLICATION_JSON.into(), json_media_type);

            let request_body = RefOr::Object(RequestBody {
                description: Some("The input value to evaluate against.".into()),
                content: content,
                required: true,
                extensions: Default::default(),
            });
            post.request_body = Some(request_body);
            post.responses = default_post_responses.clone();

            path_item.get = Some(get);
            path_item.post = Some(post);
            api.paths.insert(path, path_item);
        }
    }

    let mut components = Components::default();
    components.schemas = schemas;
    insert_default_responses(&mut components.responses);
    api.components = Some(components);

    if let Ok(api) = serde_json::to_string_pretty(&api) {
        let mut response = HttpResponse::Ok();
        response.insert_header((CONTENT_TYPE, "application/json"));
        response.body(api)
    } else {
        HttpResponse::InternalServerError().finish()
    }
}

fn insert_default_responses(responses: &mut okapi::Map<String, RefOr<Response>>) {
    let mut content = okapi::Map::new();
    content.insert(
        APPLICATION_JSON.to_string(),
        MediaType {
            ..Default::default()
        },
    );
    content.insert(
        "application/yaml".to_string(),
        MediaType {
            ..Default::default()
        },
    );
    content.insert(
        "text/html".to_string(),
        MediaType {
            ..Default::default()
        },
    );

    let ok_response = Response {
        description: "Validated successfully".into(),
        content: content.clone(),
        ..Default::default()
    };

    let nok_response = Response {
        description: "Did not validate successfully".into(),
        content,
        ..Default::default()
    };

    responses.insert(RESPONSE_SUCCESS.to_string(), RefOr::Object(ok_response));
    responses.insert(RESPONSE_FAILURE.to_string(), RefOr::Object(nok_response));
}

fn build_default_post_response() -> Responses {
    Responses {
        responses: {
            let mut responses = okapi::Map::new();
            responses.insert(
                200.to_string(),
                RefOr::Ref(Ref {
                    reference: format!("#/components/responses/{}", RESPONSE_SUCCESS),
                }),
            );
            responses.insert(
                422.to_string(),
                RefOr::Ref(Ref {
                    reference: format!("#/components/responses/{}", RESPONSE_FAILURE),
                }),
            );
            responses
        },
        ..Default::default()
    }
}

/// Translate from out examples to openapi
fn build_examples(examples: Vec<Example>) -> Option<okapi::Map<String, okapi::openapi3::Example>> {
    if examples.is_empty() {
        return None;
    }

    let mut result = okapi::Map::new();

    for example in examples {
        let ex = okapi::openapi3::Example {
            summary: example.summary,
            description: example.description,
            value: ExampleValue::Value(example.value),
            extensions: Default::default(),
        };
        result.insert(example.name, ex);
    }

    Some(result)
}
