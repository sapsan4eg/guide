use std::sync::Arc;
use std::collections::{HashMap};
use iron::{method, Handler, IronResult, Response, Request, status, IronError, Url, headers};
use iron::modifiers::Redirect;
use recognizer::Router as Recognizer;
use std::error::Error;
use std::fmt;
use iron::typemap::Key;
use recognizer::{Match, Params};
use mount;
use url;

pub trait RouteHandler: Send + Sync + 'static {
    fn handle(&self, &mut Request, &str) -> IronResult<Response>;
}

pub struct Router {
    matcher: Arc<Recognizer<HashMap<method::Method, String>>>,
    handlers: Vec<Box<RouteHandler + Send + Sync>>,
    wildcard: Recognizer<String>,
    route_ids: HashMap<String, String>,
}

impl Router {
    pub fn new() -> Router {
        Router {
            matcher: Arc::new(Recognizer::new()),
            handlers: Vec::new(),
            wildcard: Recognizer::new(),
            route_ids: HashMap::new()
        }
    }

    fn mut_matcher(&mut self) -> &mut Recognizer<HashMap<method::Method, String>> {
        Arc::get_mut(&mut self.matcher).expect("Cannot modify router at this point.")
    }

    pub fn link<T: RouteHandler>(&mut self, handler: T) -> &mut Router {
        self.handlers.push(Box::new(handler));
        self
    }

    pub fn route<S: AsRef<str>>(&mut self, method: method::Method, glob: S, route_id: &str) -> &mut Router {

        let mut hash: HashMap<method::Method, String>;

        if let Some(s) = self.mut_matcher().recognize(glob.as_ref()).ok() {
            hash = s.handler.clone();
        } else {
            hash = HashMap::new();
        }

        hash.insert(method, route_id.to_string());
        self.mut_matcher().add(glob.as_ref(), hash);
        self.route_id(route_id.as_ref(), glob.as_ref());
        self
    }

    fn route_id(&mut self, id: &str, glob: &str) {
        match self.route_ids.get(id) {
            Some(other_glob) if glob != other_glob => panic!("Duplicate route_id: {}", id),
            _ => ()
        };

        self.route_ids.insert(id.to_owned(), glob.to_owned());
    }

    pub fn get<S: AsRef<str>>(&mut self, glob: S, route_id: &str) -> &mut Router {
        self.route(method::Get, glob, route_id)
    }

    pub fn post<S: AsRef<str>>(&mut self, glob: S, route_id: &str) -> &mut Router {
        self.route(method::Post, glob, route_id)
    }

    pub fn put<S: AsRef<str>>(&mut self, glob: S, route_id: &str) -> &mut Router {
        self.route(method::Put, glob, route_id)
    }

    pub fn delete<S: AsRef<str>>(&mut self, glob: S, route_id: &str) -> &mut Router {
        self.route(method::Delete, glob, route_id)
    }

    pub fn head<S: AsRef<str>>(&mut self, glob: S, route_id: &str) -> &mut Router {
        self.route(method::Head, glob, route_id)
    }

    pub fn patch<S: AsRef<str>>(&mut self, glob: S, route_id: &str) -> &mut Router {
        self.route(method::Patch, glob, route_id)
    }

    pub fn options<S: AsRef<str>>(&mut self, glob: S, route_id: &str) -> &mut Router {
        self.route(method::Options, glob, route_id)
    }

    pub fn any<S: AsRef<str>>(&mut self, glob: S, route_id: &str) -> &mut Router {
        self.wildcard.add(glob.as_ref(), route_id.to_string());
        self.route_id(route_id.as_ref(), glob.as_ref());
        self
    }

    fn recognize(&self, method: &method::Method, path: &str) -> Result<Match<String>, RouterError> {
        match self.matcher.recognize(path)
            .map(|s|
                match s.handler.get(method) {
                    Some(h) => Ok(Match::new(h.to_string(), s.params)),
                    None => self.wildcard.recognize(path).ok()
                        .map_or(Err(RouterError::MethodNotAllowed), |s| Ok(Match::new(s.handler.to_string(), s.params)))
                }
            ).map_err(|_| self.wildcard.recognize(path).ok()
            .map_or(Err(RouterError::NotFound), |s| Ok(Match::new(s.handler.to_string(), s.params)))) {
            Ok(s) => s,
            Err(e) => e
        }
    }

    fn handlers(&self, req: &mut Request, route_id: String) -> IronResult<Response> {

        for x in &self.handlers {
            match x.handle(req, &route_id) {
                Ok(h) => {
                    return Ok(h)
                },
                Err(err) => {
                    match err.error.downcast::<RouterError>() {
                        Some(&RouterError::NextMiddleware) => {
                            continue;
                        },
                        Some(&RouterError::NotFound) => {
                            return Err(IronError::new(RouterError::NotFound, status::NotFound))
                        },
                        Some(&RouterError::TrailingSlash) => {
                            return Err(IronError::new(RouterError::TrailingSlash, status::NotFound))
                        },
                        Some(&RouterError::MethodNotAllowed) => {
                            return Err(IronError::new(RouterError::MethodNotAllowed, status::MethodNotAllowed))
                        },
                        Some(&RouterError::BadRequest) => {
                            return Err(IronError::new(RouterError::BadRequest, status::MethodNotAllowed))
                        },
                        None => {
                            return Err(err)
                        }
                    }
                }
            }
        }

        Err(IronError::new(RouterError::NotFound, status::NotFound))
    }

    fn redirect_slash(&self, req: &Request) -> Option<IronError> {
        let mut url = req.url.clone();
        let mut path = url.path().join("/");

        if let Some(original) = req.extensions.get::<mount::OriginalUrl>() {
            url =  original.clone();
        }

        if let Some(last_char) = path.chars().last() {
            // Unwrap generic URL to get access to its path components.
            let mut generic_url = url.into_generic_url();
            {
                let mut path_segments = generic_url.path_segments_mut().unwrap();
                if last_char == '/' {
                    // We didn't recognize anything without a trailing slash; try again with one appended.
                    path.pop();
                    path_segments.pop();
                } else {
                    // We didn't recognize anything with a trailing slash; try again without it.
                    path.push('/');
                    path_segments.push("");
                }
            }
            url = Url::from_generic_url(generic_url).unwrap();
        }

        self.recognize(&req.method, &path).ok().and(
            Some(IronError::new(RouterError::TrailingSlash,
                                (status::MovedPermanently, Redirect(url))))
        )
    }

    fn handle_options(&self, path: &str) -> Response {
        static METHODS: &'static [method::Method] =
        &[method::Get, method::Post, method::Put,
            method::Delete, method::Head, method::Patch];

        // Get all the available methods and return them.
        let mut options = vec![];

        for method in METHODS.iter() {
            if let Ok(s) = self.matcher.recognize(path) {
                if let Some(_) = s.handler.get(method) {
                    options.push(method.clone());
                }
            }
        }
        // If GET is there, HEAD is also there.
        if options.contains(&method::Get) && !options.contains(&method::Head) {
            options.push(method::Head);
        }

        let mut res = Response::with(status::Ok);
        res.headers.set(headers::Allow(options));
        res
    }

    fn handle_method(&self, req: &mut Request, path: &str) -> IronResult<Response> {
        match self.recognize(&req.method, &path) {
            Ok(matched) => {
                req.extensions.insert::<Router>(matched.params);
                req.extensions.insert::<RouteMap>(self.route_ids.clone());
                self.handlers(req, matched.handler)
            },
            Err(RouterError::MethodNotAllowed) => {
                Err(IronError::new(RouterError::MethodNotAllowed, status::MethodNotAllowed))
            },
            Err(_) => {
                match self.redirect_slash(req) {
                    Some(err) => Err(err),
                    None => //Err(IronError::new(RouterError::NotFound, status::NotFound))
                        match req.method {
                            method::Options => Ok(self.handle_options(&path)),
                            // For HEAD, fall back to GET. Hyper ensures no response body is written.
                            method::Head => {
                                req.method = method::Get;
                                self.handle_method(req, path)
                            },
                            _ => Err(IronError::new(RouterError::NotFound, status::NotFound))
                        }
                }
            }
        }
    }
}

pub fn get_parameter(req: &mut Request, str: &str) -> String {
    req.extensions.get::<Router>().unwrap_or(&Params::new()).find(str).unwrap_or("").to_string()
}

pub fn requested_url(req: &mut Request) -> url::Url {
    match req.extensions.get::<mount::OriginalUrl>() {
        Some(original) => original.clone().into_generic_url(),
        None => req.url.clone().into_generic_url()
    }
}

impl Handler for Router {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let path = req.url.path().join("/");
        self.handle_method(req, &path)
    }
}

impl Key for Router { type Value = Params; }

pub struct RouteMap;
impl Key for RouteMap { type Value = HashMap<String, String>; }

#[derive(Debug, PartialEq)]
pub enum RouterError {
    /// The error thrown by router if there is no matching method in existing route.
    MethodNotAllowed,
    /// The error thrown by router if there is no matching route.
    NotFound,
    /// The error thrown by router if a request was redirected by adding or removing a trailing slash.
    TrailingSlash,
    /// This middleware not support that route_id
    NextMiddleware,
    ///
    BadRequest
}


impl fmt::Display for RouterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.description())
    }
}

impl Error for RouterError {
    fn description(&self) -> &str {
        match *self {
            RouterError::MethodNotAllowed => "Method Not Allowed",
            RouterError::NotFound => "No matching route found.",
            RouterError::TrailingSlash => "The request had a trailing slash.",
            RouterError::NextMiddleware => "This is middleware not support this request",
            RouterError::BadRequest => "This is not valid request"
        }
    }
}

#[cfg(test)]
mod test {
    use super::{Router, RouterError, RouteHandler};
    use iron::{headers, method, status, Request, Response, IronError, IronResult};

    struct DummyController;

    impl RouteHandler for DummyController {
        fn handle(&self, _: &mut Request, route_id: &str) -> IronResult<Response> {
            match route_id {
                "handler" => Ok(Response::with((status::Ok, "OK all right"))),
                "someone" => Ok(Response::with((status::Ok, "OK someone"))),
                _ => {
                    Err(IronError::new(RouterError::NextMiddleware, status::Ok))
                }
            }
        }
    }

    struct DummySecondController;

    impl RouteHandler for DummySecondController {
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

    #[test]
    fn test_handle_options_post() {
        let mut router = Router::new();
        router.link(DummyController);
        router.post("/", "handler");
        let resp = router.handle_options("/");
        let headers = resp.headers.get::<headers::Allow>().unwrap();
        let expected = headers::Allow(vec![method::Method::Post]);
        assert_eq!(&expected, headers);
    }

    #[test]
    fn test_handle_options_get_head() {
        let mut router = Router::new();
        router.link(DummySecondController);
        router.get("/", "anys");
        let resp = router.handle_options("/");
        let headers = resp.headers.get::<headers::Allow>().unwrap();
        let expected = headers::Allow(vec![method::Method::Get, method::Method::Head]);
        assert_eq!(&expected, headers);
    }
    #[test]
    fn test_not_allowed_method() {
        let mut router = Router::new();
        router.link(DummyController);
        router.post("/post","someone");
        router.get("/post/", "another_route");
        match router.recognize(&method::Get, "/post") {
            Ok(_) => {
               panic!();
            },
            Err(e) => {
               assert_eq!(RouterError::MethodNotAllowed, e);
            }
        }
    }

    #[test]
    fn test_handle_any_ok() {
        let mut router = Router::new();
        router.link(DummySecondController);
        router.post("/post", "anys");
        router.any("/post", "anys");
        router.put("/post", "anys");
        router.any("/get", "another");

        assert!(router.recognize(&method::Get, "/post").is_ok());
        assert!(router.recognize(&method::Get, "/get").is_ok());
    }

    #[test]
    fn test_request() {
        let mut router = Router::new();
        router.link(DummyController);
        router.post("/post", "handler");
        router.get("/post", "someone");

        assert!(router.recognize(&method::Post, "/post").is_ok());
        assert!(router.recognize(&method::Get, "/post").is_ok());
        assert!(router.recognize(&method::Put, "/post").is_err());
        assert!(router.recognize(&method::Get, "/post/").is_err());
    }

    #[test]
    fn test_not_found() {
        let mut router = Router::new();
        router.link(DummyController);

        router.put("/put", "handler");
        match router.recognize(&method::Patch, "/patch") {
            Ok(_) => {
                panic!();
            },
            Err(e) => {
                assert_eq!(RouterError::NotFound, e);
            }
        }
    }

    #[test]
    #[should_panic]
    fn test_same_route_id() {
        let mut router = Router::new();
        router.put("/put", "my_route_id");
        router.get("/get", "my_route_id");
    }
}