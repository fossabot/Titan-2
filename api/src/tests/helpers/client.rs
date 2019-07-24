use crate::server;
use rocket::{
    http::{Header, Status},
    local::{Client as RocketClient, LocalResponse as RocketResponse},
};
use serde_json::Value;

pub struct Client<'a> {
    base:   &'a str,
    client: RocketClient,
}

impl<'a> Client<'a> {
    pub fn new() -> Self {
        Client {
            base:   "",
            client: RocketClient::new(server()).expect("invalid rocket instance"),
        }
    }

    pub fn with_base(&mut self, base: &'a str) -> &Self {
        self.base = base;
        self
    }

    fn url_for(&self, id: impl ToString) -> String {
        format!("{}/{}", self.base, id.to_string())
    }

    pub fn get_all(&self) -> Response<'_> {
        self.get("")
    }

    pub fn get(&self, id: impl ToString) -> Response<'_> {
        Response(self.client.get(self.url_for(id)).dispatch())
    }

    pub fn post(&self, token: Option<&str>, body: impl ToString) -> Response<'_> {
        Response(match token {
            Some(token) => self
                .client
                .post(self.base)
                .body(body.to_string())
                .header(Header::new("Authorization", format!("Bearer {}", token)))
                .dispatch(),
            None => self
                .client
                .post(self.base)
                .body(body.to_string())
                .dispatch(),
        })
    }

    pub fn patch(
        &self,
        token: Option<&str>,
        id: impl ToString,
        body: impl ToString,
    ) -> Response<'_> {
        Response(match token {
            Some(token) => self
                .client
                .patch(self.url_for(id))
                .body(body.to_string())
                .header(Header::new("Authorization", format!("Bearer {}", token)))
                .dispatch(),
            None => self
                .client
                .patch(self.url_for(id))
                .body(body.to_string())
                .dispatch(),
        })
    }

    pub fn delete(&self, token: Option<&str>, id: impl ToString) -> Response<'_> {
        Response(match token {
            Some(token) => self
                .client
                .delete(self.url_for(id))
                .header(Header::new("Authorization", format!("Bearer {}", token)))
                .dispatch(),
            None => self.client.delete(self.url_for(id)).dispatch(),
        })
    }
}

#[derive(Debug)]
pub struct Response<'a>(RocketResponse<'a>);
impl Response<'_> {
    fn status(&self) -> Status {
        self.0.status()
    }

    pub fn assert_ok(self) -> Self {
        assert_eq!(self.status(), Status::Ok);
        self
    }

    pub fn assert_created(self) -> Self {
        assert_eq!(self.status(), Status::Created);
        self
    }

    pub fn assert_no_content(self) -> Self {
        assert_eq!(self.status(), Status::NoContent);
        self
    }

    pub fn assert_see_other(self) -> Self {
        assert_eq!(self.status(), Status::SeeOther);
        self
    }

    pub fn get_redirect_uri(self) -> String {
        self.0.headers().get_one("Location").unwrap().into()
    }

    pub fn get_body_array(mut self) -> Value {
        let body = self.body();
        assert!(body.is_array(), "body is array");
        body
    }

    pub fn get_body_object(mut self) -> Value {
        let body = self.body();
        assert!(body.is_object(), "body is object");
        body
    }

    fn body(&mut self) -> Value {
        self.0
            .body_string()
            .map(|body| serde_json::from_str(&body))
            .unwrap()
            .unwrap()
    }
}
