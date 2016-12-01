extern crate iron;
extern crate guide;
extern crate mount;

use iron::{Iron, Request, Response, IronResult, method, IronError, status};
use guide::{Router, RouteHandler, RouterError};
use std::collections::HashMap;
use mount::Mount;

struct DummyController;
struct DummyTwoController;

impl RouteHandler for DummyController {
    fn handle(&self, req: &mut Request, route_id: &str) -> IronResult<Response> {
        match route_id {
            "handler" => Ok(Response::with((status::Ok, "OK all right"))),
            "someone" => {
                let mut params =  HashMap::new();
                params.insert("everybody".to_string(), "cool".to_string());

                let ref query = req.extensions.get::<Router>()
                    .unwrap().find("everybody").unwrap_or("/");
                Ok(Response::with((status::Ok, format!("Ok someone {:?} {:?}", query, guide::url_for(req, route_id,params)))))
            },
            _ => {
                Err(IronError::new(RouterError::NextMiddleware, status::Ok))
            }
        }
    }
}

impl RouteHandler for DummyTwoController {
    fn handle(&self, _: &mut Request, route_id: &str) -> IronResult<Response> {
        match route_id {
            "another" => Ok(Response::with((status::Ok, "OK another"))),
            "anys" => Ok(Response::with((status::Ok, "OK anys"))),
            _ => {
                Err(IronError::new(RouterError::NextMiddleware, status::Ok))
            }
        }
    }
}

fn main() {
    let mut mount = Mount::new();
    let mut router = Router::new();
    router.link(DummyController);
    router.link(DummyTwoController);
    router.route(method::Get, "/", "handler");
    router.post("/hello", "another");
    router.get("/hi/:everybody", "someone");
    router.any("/hello", "anys");
    mount
        .mount("api", router);
    Iron::new(mount).http("localhost:3000").unwrap();
}

