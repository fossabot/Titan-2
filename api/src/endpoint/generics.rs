/// Generate an endpoint for fetching all of the provided type.
///
/// This macro should suffice for all types.
#[macro_export]
macro_rules! generic_all {
    ($x:ident) => {
        #[rocket::get("/")]
        pub fn all(
            conn: crate::DataDB,
        ) -> crate::endpoint::helpers::RocketResult<rocket_contrib::json::Json<Vec<$x>>> {
            json_result!($x::find_all(&conn))
        }
    };
}

/// Generate an endpoint for fetching a specific instance of the provided type.
///
/// This macro should suffice for all types.
#[macro_export]
macro_rules! generic_get {
    ($x:ident) => {
        #[rocket::get("/<id>")]
        pub fn get(
            conn: crate::DataDB,
            id: i32,
        ) -> crate::endpoint::helpers::RocketResult<rocket_contrib::json::Json<$x>> {
            json_result!($x::find_id(&conn, id))
        }
    };
}
