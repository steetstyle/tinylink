use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tinylink::code::route::{RouteEntry, RouteTable};
use tinylink::code::types::{CodeGraph, MethodNode};
use tinylink::sdg::types::ServiceGraph;

fn build_test_data(size: usize) -> (ServiceGraph, CodeGraph, RouteTable) {
    let mut sdg = ServiceGraph::new();
    let mut code = CodeGraph::new();
    let mut route = RouteTable::new();

    for i in 0..size {
        let svc = format!("Svc{}", i);
        let class = format!("Controller{}", i);
        let method = format!("GetItem{}", i);
        sdg.add_edge(
            &svc,
            &format!("Svc{}", i + 1),
            &format!("GET /api/{}/items", i),
            "POST /validate",
            false,
            100.0,
            "trace",
        );
        code.add_method(MethodNode {
            class: class.clone(),
            method: method.clone(),
            signature: format!("Item {}()", method),
            file: format!("{}.cs", class),
            line_start: 1,
            line_end: 10,
            service: Some(svc.clone()),
        });
        route.add(RouteEntry {
            http_method: "GET".into(),
            route_pattern: format!("/api/{}/items", i),
            class,
            method,
        });
    }

    (sdg, code, route)
}

fn criterion_benchmark(c: &mut Criterion) {
    let (sdg_small, code_small, route_small) = build_test_data(5);
    let (sdg_med, code_med, route_med) = build_test_data(20);

    c.bench_function("unify_small_5_services", |b| {
        b.iter(|| {
            let _ = tinylink::link::mapper::build_unified_from_sdg(
                black_box(&sdg_small),
                black_box(&code_small),
                black_box(&route_small),
            );
        })
    });

    c.bench_function("unify_medium_20_services", |b| {
        b.iter(|| {
            let _ = tinylink::link::mapper::build_unified_from_sdg(
                black_box(&sdg_med),
                black_box(&code_med),
                black_box(&route_med),
            );
        })
    });

    c.bench_function("route_match_template", |b| {
        let _pattern = "/api/items/{id}/details/{locale}";
        let path = "/api/items/42/details/en-US";
        b.iter(|| {
            let route = tinylink::code::route::RouteTable::new();
            let _ = route.match_best("GET", black_box(path));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
