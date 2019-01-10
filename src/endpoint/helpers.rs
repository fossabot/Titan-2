use rocket::http::Status;
use rocket_contrib::databases::diesel::result::Error;

pub type RocketResult<T> = Result<T, Status>;

#[inline]
pub fn error_mapper(err: Error) -> Status {
    match err {
        Error::NotFound => Status::NotFound,
        _ => Status::InternalServerError,
    }
}

#[macro_export]
macro_rules! json_result {
    ($x:expr) => {
        $x.map(rocket_contrib::json::Json)
            .map_err(crate::endpoint::helpers::error_mapper)
    };
}

#[macro_export]
macro_rules! no_content {
    ($x:expr) => {
        $x.map(|_| rocket::http::Status::NoContent)
            .map_err(crate::endpoint::helpers::error_mapper)
    };
}

#[macro_export]
macro_rules! created {
    ($x:expr) => {
        $x
            .map(|value| rocket::response::status::Created(
                uri!(get: value.id).to_string(),
                Some(rocket_contrib::json::Json(value))
            ))
            .map_err(crate::endpoint::helpers::error_mapper)
    };
}
