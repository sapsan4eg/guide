Guide [![Build Status](https://travis-ci.org/sapsan4eg/guide.svg?branch=master)](https://travis-ci.org/iron/router)
===================================================================================================================

> Simple router for the [Iron](https://github.com/iron/iron) web framework.

Hello everybody, this is simple library based on [iron-router]https://github.com/iron/router
I love [Iron] and they philosophy. But for me iron-router not quite right.
Main idea in iron-router is one route (url path) to one closure or instance of the Struct. It's work very well.
But for me more familiar many routes (url paths) to one instance of the Struct.
I use `chain or the responsibility` and every Structs must implement `RouteHandler`
and return `Err(IronError::new(RouterError::NextMiddleware, status::Ok))` 
if the route does not belong to this `RouteHandler`

## Example

```rust
extern crate iron;
extern crate guide;

use iron::{Iron, Request, Response, IronResult, method, IronError, status};
use guide::{Router, RouteHandler, RouterError};
use std::collections::HashMap;

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
    let mut router = Router::new();
    router.link(DummyController);
    router.link(DummyTwoController);
    router.route(method::Get, "/", "handler");
    router.route(method::Post, "/hello", "another");
    router.route(method::Get, "/hi/:everybody", "someone");
    router.any("/hello", "anys");

    Iron::new(router).http("localhost:3000").unwrap();
}
```

## Installation

If you're using cargo, just add router to your `Cargo.toml`.

```toml
[dependencies.guide]
git = "https://github.com/sapsan4eg/guide"
version = "0.1.0"
```

Otherwise, `cargo build`, and the rlib will be in your `target` directory.

## [Examples](/examples)
