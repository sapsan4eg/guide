use std::collections::HashMap;

use url::Url;

use iron::prelude::*;
use super::router::RouteMap;
use mount;

/// Generate a URL based off of the currently requested URL.
///
/// The `route_id` used during route registration will be used here again.
///
/// `params` will be inserted as route parameters if fitting, the rest will be appended as query
/// parameters.
pub fn url_for(request: &Request, route_id: &str, params: HashMap<String, String>) -> ::iron::Url {
    let map = request.extensions.get::<RouteMap>().expect("Couldn\'t find router set up properly.");
    let glob = map.get(route_id).expect("No route with that ID");

    let mut url;
    let mut base_path: String = "".to_string();

    if let Some(original) = request.extensions.get::<mount::OriginalUrl>() {
        url =  original.clone().into_generic_url();
        let routed = request.url.clone().into_generic_url();
        base_path = take_base_path(url.path(), routed.path());
    } else {
        url = request.url.clone().into_generic_url();
    }

    url_for_impl(&mut url, glob, params, base_path);
    ::iron::Url::from_generic_url(url).unwrap()
}

fn take_base_path(requested: &str, routed: &str) -> String {

    let mut s: String = "".to_string();

    if requested == routed {
        return s
    }

    let route: Vec<&str> = routed.split('/').collect();
    let req: Vec<&str> = requested.split('/').collect();

    let mut i = 0;

    for t in req {
        if route.len() - 1 == i {
            break
        }
        if t == "" {
            i += 1;
            continue
        }

        if t == route[i] {
            break
        } else {
            s.push_str(&format!("{}/", t));
        }

        i += 1;
    }

    s
}

fn url_for_impl(url: &mut Url, glob: &str, mut params: HashMap<String, String>, base_path: String) {
    {
        let mut url_path_segments = url.path_segments_mut().unwrap();
        url_path_segments.clear();
        for base in base_path.split('/') {
            if base != "" {
                url_path_segments.push(base);
            }
        }
        let mut first_slash: bool = true;
        for path_segment in glob.split('/') {
            if path_segment.len() > 1 && (path_segment.starts_with(':') || path_segment.starts_with('*')) {
                let key = &path_segment[1..];
                match params.remove(key) {
                    Some(x) => url_path_segments.push(&x),
                    None => panic!("No value for key {}", key)
                };
            } else {
                if first_slash == true && path_segment == "" {
                    first_slash = false;
                } else {
                    url_path_segments.push(path_segment);
                }
            }
        }
    }

    // Now add on the remaining parameters that had no path match.
    url.set_query(None);
    if !params.is_empty() {
        url.query_pairs_mut()
            .extend_pairs(params.into_iter());
    }

    url.set_fragment(None);
}

#[cfg(test)]
mod test {
    use super::{url_for_impl, take_base_path};
    use std::collections::HashMap;

    #[test]
    fn test_no_trailing_slash() {
        let mut url = "http://localhost/foo/bar/baz".parse().unwrap();
        url_for_impl(&mut url, "/foo/:user", {
            let mut rv = HashMap::new();
            rv.insert("user".into(), "bam".into());
            rv
        }, "".to_string());
        assert_eq!(url.to_string(), "http://localhost/foo/bam");
    }

    #[test]
    fn test_no_trailing_slash_second() {
        let mut url = "http://localhost/foo/bar".parse().unwrap();
        url_for_impl(&mut url, "/foo/:user", {
            let mut rv = HashMap::new();
            rv.insert("user".into(), "bam".into());
            rv
        }, "".to_string());
        assert_eq!(url.to_string(), "http://localhost/foo/bam");
    }

    #[test]
    fn test_trailing_slash() {
        let mut url = "http://localhost/foo/bar/baz".parse().unwrap();
        url_for_impl(&mut url, "/foo/:user/", {
            let mut rv = HashMap::new();
            rv.insert("user".into(), "bam".into());
            rv
        }, "".to_string());
        assert_eq!(url.to_string(), "http://localhost/foo/bam/");
    }

    #[test]
    fn test_with_mount() {
        let mut url = "http://localhost/mounted/foo/bar/wert".parse().unwrap();
        url_for_impl(&mut url, "/foo/:user/", {
            let mut rv = HashMap::new();
            rv.insert("user".into(), "bam".into());
            rv
        }, "mounted/".to_string());
        assert_eq!(url.to_string(), "http://localhost/mounted/foo/bam/");
    }

    #[test]
    fn test_with_mount_second() {
        let mut url = "http://localhost/mounted/foo/bar".parse().unwrap();
        url_for_impl(&mut url, "/foo/:user", {
            let mut rv = HashMap::new();
            rv.insert("user".into(), "bam".into());
            rv
        }, "/mounted/".to_string());
        assert_eq!(url.to_string(), "http://localhost/mounted/foo/bam");
    }

    #[test]
    fn test_take_base_path() {
        let s = take_base_path("/mounted/foo/bar", "/foo/:biz");
        assert_eq!(s, "mounted/");
    }
}
