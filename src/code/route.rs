use serde::{Serialize, Deserialize};

/// A parsed route entry from C# source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    pub http_method: String,
    pub route_pattern: String,
    pub class: String,
    pub method: String,
}

/// Route table mapping HTTP routes to methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteTable {
    pub entries: Vec<RouteEntry>,
}

impl RouteTable {
    pub fn new() -> Self {
        RouteTable { entries: Vec::new() }
    }

    pub fn add(&mut self, entry: RouteEntry) {
        self.entries.push(entry);
    }

    /// Match an HTTP method+path against all registered routes.
    /// Supports template parameters like {id}, {name}.
    pub fn match_route(&self, http_method: &str, path: &str) -> Vec<&RouteEntry> {
        let path_lower = path.to_lowercase();
        self.entries.iter()
            .filter(|e| {
                e.http_method.eq_ignore_ascii_case(http_method)
                    && match_template(&e.route_pattern, &path_lower)
            })
            .collect()
    }

    /// Find the closest matching route by segment count (most specific)
    pub fn match_best(&self, http_method: &str, path: &str) -> Option<&RouteEntry> {
        let matches = self.match_route(http_method, path);
        if matches.is_empty() {
            return None;
        }
        // Return the most specific match (most segments)
        matches.into_iter()
            .max_by_key(|e| e.route_pattern.matches('/').count())
    }
}

/// Check if a route pattern matches a request path.
/// Pattern: "/api/items/{id}" matches "/api/items/123"
/// Pattern: "/api/items" matches "/api/items"
fn match_template(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim_end_matches('/');
    let path = path.trim_end_matches('/');

    let p_segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let r_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if p_segments.len() != r_segments.len() {
        return false;
    }

    for (p, r) in p_segments.iter().zip(r_segments.iter()) {
        if p.starts_with('{') && p.ends_with('}') {
            // Template parameter — matches anything
            continue;
        }
        if !p.eq_ignore_ascii_case(r) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(match_template("/api/items", "/api/items"));
    }

    #[test]
    fn test_template_match() {
        assert!(match_template("/api/items/{id}", "/api/items/123"));
    }

    #[test]
    fn test_template_match_multiple() {
        assert!(match_template("/api/{category}/{id}", "/api/books/42"));
    }

    #[test]
    fn test_no_match_different_segments() {
        assert!(!match_template("/api/items", "/api/items/123"));
    }

    #[test]
    fn test_no_match_different_path() {
        assert!(!match_template("/api/items", "/api/users"));
    }

    #[test]
    fn test_case_insensitive() {
        assert!(match_template("/API/Items", "/api/items"));
    }

    #[test]
    fn test_route_table_best_match_prefers_longer() {
        let mut table = RouteTable::new();
        table.add(RouteEntry {
            http_method: "GET".into(),
            route_pattern: "/api/{resource}".into(),
            class: "GenericHandler".into(),
            method: "Handle".into(),
        });
        table.add(RouteEntry {
            http_method: "GET".into(),
            route_pattern: "/api/items".into(),
            class: "ItemsController".into(),
            method: "GetAll".into(),
        });
        let best = table.match_best("GET", "/api/items");
        assert!(best.is_some());
        assert_eq!(best.unwrap().class, "ItemsController");
    }

    #[test]
    fn test_route_table_no_match() {
        let table = RouteTable::new();
        assert!(table.match_best("POST", "/api/items").is_none());
    }

    #[test]
    fn test_match_with_query_string() {
        // Query strings not part of route pattern matching
        let path = "/api/items?page=1";
        let clean = path.split('?').next().unwrap_or(path);
        assert!(match_template("/api/items", clean));
    }

    #[test]
    fn test_trailing_slashes() {
        assert!(match_template("/api/items/", "/api/items"));
        assert!(match_template("/api/items", "/api/items/"));
    }
}
