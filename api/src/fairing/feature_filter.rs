use hashbrown::HashSet;
use rocket::{
    fairing::{Fairing, Info, Kind},
    Request,
    Response,
};
use serde_json::{Map, Value as Json};
use std::io::Cursor;

/// Remove any feature-specific fields unless requested.
///
/// A feature-specific field is one whose key contains two consecutive underscores.
/// All characters preceding those underscores is the feature name.
///
/// To enable a feature,
/// you should use the `feature` queryparam,
/// passing a comma separated list of features to enable.
///
/// By default, no features are enabled.
///
/// Usage:
/// ```rust
/// rocket::ignite.attach(FeatureFilter::default()).launch()
/// ```
#[derive(Default, Debug)]
pub struct FeatureFilter;

impl Fairing for FeatureFilter {
    /// Give Rocket some information about the fairing, including when to call it.
    fn info(&self) -> Info {
        Info {
            name: "Feature filter",
            kind: Kind::Response,
        }
    }

    /// After a request is completed,
    /// call `filter_array` and `filter_object` as necessary to remove any unwanted fields.
    ///
    /// FIXME Is there any valid use case for an "all" feature flag?
    fn on_response(&self, request: &Request<'_>, response: &mut Response<'_>) {
        if let Some(body_string) = response.body_string() {
            if let Ok(mut body) = serde_json::from_str(&body_string) {
                let features_str = request
                    .get_query_value("features")
                    .unwrap_or_else(|| Ok("".to_string()))
                    .unwrap()
                    .to_lowercase();
                let features: HashSet<&str> = features_str.split(',').collect();

                designator(&mut body, &features);
                response.set_sized_body(Cursor::new(body.to_string()));
            } else {
                response.set_sized_body(Cursor::new(body_string));
            };
        } else {
            // Error converting the body to a String;
            // there aren't any fields to remove.
        }
    }
}

/// Call `filter_object` and `filter_array` as appropriate.
fn designator(value: &mut Json, features: &HashSet<&str>) {
    if value.is_object() {
        filter_object(value.as_object_mut().unwrap(), features);
    } else if value.is_array() {
        filter_array(value.as_array_mut().unwrap(), features);
    }
}

/// Recursively filter the fields of an object in-place.
fn filter_object(object: &mut Map<String, Json>, features: &HashSet<&str>) {
    for (key, _) in object.clone().iter() {
        let value = &mut object[key];

        // Recursively reach each value.
        designator(value, features);

        // This field requires a feature that wasn't requested.
        if key.contains("__")
            && !features.contains(&*key.splitn(2, "__").next().unwrap().to_lowercase())
        {
            object.remove(key);
        }
    }
}

/// Recursively filter the fields of any child objects of an array in-place.
fn filter_array(array: &mut Vec<Json>, features: &HashSet<&str>) {
    for value in array {
        designator(value, features);
    }
}
