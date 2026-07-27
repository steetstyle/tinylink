#!/usr/bin/env python3
"""eShopOnContainers service graph metrics exporter.

Serves Prometheus histogram metrics that exactly match
eShopOnContainers service interactions with correct
endpoint paths, service names, and realistic latency distributions.

Services: webmvc, catalog.api, ordering.api, basket.api, payment.api, identity.api
DBs: catalog-db, ordering-db, basket-data, identity-db
Messaging: rabbitmq
gRPC: BasketService, CatalogService
"""

import http.server
import random
import time
from typing import Dict, List, Tuple

HOST = "0.0.0.0"
PORT = 9464

BUCKETS = [0.002, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, "+Inf"]

# ── eShopOnContainers service interactions ─────────────────────────
# (client, server, http_method, route, avg_ms, error_pct, volume)
# Routes match actual ASP.NET Core controller routes exactly.
# Includes HTTP REST, gRPC, DB, and message broker interactions.

INTERACTIONS: List[Tuple] = [
    # ═══ webmvc → catalog.api ═══
    ("webmvc", "catalog.api",      "GET",    "/api/v1/catalog/items",                       35,   1, 800),
    ("webmvc", "catalog.api",      "GET",    "/api/v1/catalog/items/{id}",                  22,   1, 500),
    ("webmvc", "catalog.api",      "GET",    "/api/v1/catalog/items/withname/{name}",       45,   2, 150),
    ("webmvc", "catalog.api",      "GET",    "/api/v1/catalog/catalogbrands",               30,   1, 60),
    ("webmvc", "catalog.api",      "GET",    "/api/v1/catalog/catalogtypes",                28,   1, 60),
    ("webmvc", "catalog.api",      "GET",    "/api/v1/catalog/items/type/{typeId}/brand/{brandId}", 55, 2, 100),
    ("webmvc", "catalog.api",      "POST",   "/api/v1/catalog/items",                      110,   3, 40),
    ("webmvc", "catalog.api",      "PUT",    "/api/v1/catalog/items",                      100,   2, 30),

    # ═══ webmvc → ordering.api ═══
    ("webmvc", "ordering.api",     "GET",    "/api/v1/orders",                              55,   2, 300),
    ("webmvc", "ordering.api",     "GET",    "/api/v1/orders/{id}",                         38,   1, 250),
    ("webmvc", "ordering.api",     "POST",   "/api/v1/orders",                              320,  5, 120),
    ("webmvc", "ordering.api",     "PUT",    "/api/v1/orders/{id}/cancel",                  180,  8, 40),
    ("webmvc", "ordering.api",     "PUT",    "/api/v1/orders/{id}/ship",                    150,  3, 25),
    ("webmvc", "ordering.api",     "GET",    "/api/v1/orders/cardtypes",                    20,   0, 80),

    # ═══ webmvc → basket.api ═══
    ("webmvc", "basket.api",       "GET",    "/api/v1/basket/{id}",                         18,   1, 600),
    ("webmvc", "basket.api",       "POST",   "/api/v1/basket",                              140,  4, 180),
    ("webmvc", "basket.api",       "DELETE", "/api/v1/basket/{id}",                         22,   1, 90),

    # ═══ webmvc → identity.api ═══
    ("webmvc", "identity.api",     "POST",   "/connect/token",                              75,   3, 700),
    ("webmvc", "identity.api",     "GET",    "/connect/userinfo",                           25,   1, 400),
    ("webmvc", "identity.api",     "POST",   "/connect/register",                           90,   2, 80),

    # ═══ webmvc → webhooks (optional) ═══
    ("webmvc", "webhooks.api",     "POST",   "/api/v1/webhooks/checkout",                   200,  6, 15),
    ("webmvc", "webhooks.api",     "GET",    "/api/v1/webhooks/receivers",                  30,   1, 20),

    # ═══ ordering.api → basket.api (gRPC) ═══
    ("ordering.api", "basket.api", "POST",   "/BasketService/GetBasketByCustomerId",        28,   2, 200),
    ("ordering.api", "basket.api", "POST",   "/BasketService/UpdateBasket",                 45,   3, 80),

    # ═══ ordering.api → payment.api ═══
    ("ordering.api", "payment.api","POST",   "/api/v1/payment",                             1200, 10, 80),
    ("ordering.api", "payment.api","GET",    "/api/v1/payment/{id}",                        70,   2, 120),

    # ═══ ordering.api → ordering-db (SQL Server) ═══
    ("ordering.api", "ordering-db","POST",   "INSERT INTO orders",                          15,   1, 500),
    ("ordering.api", "ordering-db","POST",   "INSERT INTO orderItems",                      10,   1, 800),
    ("ordering.api", "ordering-db","POST",   "UPDATE orders SET status",                    18,   2, 350),
    ("ordering.api", "ordering-db","POST",   "SELECT orders WHERE",                         8,    1, 600),

    # ═══ basket.api → catalog.api (gRPC) ═══
    ("basket.api", "catalog.api",  "POST",   "/CatalogService/GetItemById",                 15,   1, 300),
    ("basket.api", "catalog.api",  "POST",   "/CatalogService/GetItemsByType",              20,   1, 100),

    # ═══ basket.api → basket-data (Redis) ═══
    ("basket.api", "basket-data",  "POST",   "REDIS GET basket:{id}",                       2,    0, 700),
    ("basket.api", "basket-data",  "POST",   "REDIS SET basket:{id}",                       3,    0, 500),
    ("basket.api", "basket-data",  "POST",   "REDIS DEL basket:{id}",                       1,    0, 80),

    # ═══ catalog.api → catalog-db (PostgreSQL) ═══
    ("catalog.api",  "catalog-db", "POST",   "SELECT FROM catalog",                         6,    1, 1200),
    ("catalog.api",  "catalog-db", "POST",   "SELECT FROM catalog WHERE Id",                4,    1, 800),
    ("catalog.api",  "catalog-db", "POST",   "INSERT INTO catalog",                         10,   2, 100),
    ("catalog.api",  "catalog-db", "POST",   "UPDATE catalog SET",                          12,   2, 60),

    # ═══ identity.api → identity-db (SQL Server) ═══
    ("identity.api", "identity-db","POST",   "SELECT FROM AspNetUsers WHERE",                6,    1, 600),
    ("identity.api", "identity-db","POST",   "INSERT INTO AspNetUsers",                      12,   2, 120),
    ("identity.api", "identity-db","POST",   "UPDATE AspNetUsers SET",                       10,   2, 80),

    # ═══ payment.api → payment-db ═══
    ("payment.api",  "payment-db", "POST",   "INSERT INTO payments",                         8,    1, 100),
    ("payment.api",  "payment-db", "POST",   "SELECT FROM payments WHERE",                   5,    1, 150),

    # ═══ RabbitMQ messaging (producer side) ═══
    ("ordering.api", "rabbitmq",   "POST",   "PUBLISH order.created",                       3,    0, 400),
    ("ordering.api", "rabbitmq",   "POST",   "PUBLISH order.status_changed",                 2,    0, 300),
    ("basket.api",   "rabbitmq",   "POST",   "PUBLISH basket.updated",                      3,    0, 200),
    ("payment.api",  "rabbitmq",   "POST",   "PUBLISH payment.succeeded",                    2,    0, 150),
    ("payment.api",  "rabbitmq",   "POST",   "PUBLISH payment.failed",                      2,    0, 80),

    # ═══ RabbitMQ messaging (consumer side — server kind) ═══
    ("rabbitmq",     "ordering.api","POST",  "CONSUME order.created",                       8,    1, 380),
    ("rabbitmq",     "ordering.api","POST",  "CONSUME order.status_changed",                6,    1, 280),
    ("rabbitmq",     "basket.api",  "POST",  "CONSUME basket.updated",                      5,    1, 190),
    ("rabbitmq",     "payment.api", "POST",  "CONSUME payment.succeeded",                   8,    1, 140),
    ("rabbitmq",     "payment.api", "POST",  "CONSUME payment.failed",                      5,    2, 70),

    # ═══ SignalR / WebSocket notifications ═══
    ("ordering.api", "webmvc",     "POST",   "/hub/notifications",                          15,   0, 350),
    ("ordering.api", "webmvc",     "POST",   "/hub/order-updates",                          12,   0, 250),
]


def generate_histogram(client: str, server: str, http_method: str,
                       http_route: str, avg_ms: float, error_pct: float,
                       volume: int) -> str:
    """Generate Prometheus histogram text for one service interaction.

    Uses exponential distribution to create realistic bucket counts.
    Error requests are distributed identically but summed into the
    cumulative bucket total (as OpenTelemetry does).
    """
    avg_sec = avg_ms / 1000.0
    error_count = int(volume * error_pct / 100.0)
    ok_count = volume - error_count
    total = volume

    base_labels = f'client="{client}",server="{server}",http_method="{http_method}",http_route="{http_route}"'

    # Distribute counts across latency buckets using exponential CDF
    cumul = 0
    remaining_ok = ok_count
    remaining_err = error_count
    lines = []

    for bucket_idx, bucket in enumerate(BUCKETS):
        if bucket == "+Inf":
            ok_here = remaining_ok
            err_here = remaining_err
        else:
            ratio = 1.0 - (2.71828 ** (-bucket / (avg_sec * 0.6 + 0.0001)))
            target = int(total * ratio)
            if bucket_idx == 0:
                ok_here = max(0, int(ok_count * ratio))
                err_here = max(0, int(error_count * ratio))
            else:
                prev_ratio = 1.0 - (2.71828 ** (-BUCKETS[bucket_idx - 1] / (avg_sec * 0.6 + 0.0001)))
                ok_here = max(0, int(ok_count * ratio) - int(ok_count * prev_ratio))
                err_here = max(0, int(error_count * ratio) - int(error_count * prev_ratio))

            ok_here = min(ok_here, remaining_ok)
            err_here = min(err_here, remaining_err)

        cumul += ok_here + err_here
        remaining_ok -= ok_here
        remaining_err -= err_here
        if remaining_ok < 0:
            remaining_ok = 0
        if remaining_err < 0:
            remaining_err = 0

        labels = f'{base_labels},le="{bucket}"'
        lines.append(f'traces_service_graph_request_server_bucket{{{labels}}} {cumul}')

    total_sum_sec = ok_count * avg_sec + error_count * avg_sec * 1.5
    lines.append(f'traces_service_graph_request_server_count{{{base_labels}}} {total}')
    lines.append(f'traces_service_graph_request_server_sum{{{base_labels}}} {total_sum_sec:.4f}')

    return "\n".join(lines)


class MetricsHandler(http.server.BaseHTTPRequestHandler):
    """Serves Prometheus-format metrics at /metrics"""

    def do_GET(self):
        if self.path != "/metrics":
            self.send_response(404)
            self.end_headers()
            return

        noise_seed = time.time()
        random.seed(int(noise_seed))

        lines = [
            '# HELP traces_service_graph_request_server_bucket Service graph edge latency histogram (eShopOnContainers)',
            '# TYPE traces_service_graph_request_server_bucket histogram',
        ]

        for it in INTERACTIONS:
            client, server, method, route, avg_ms, err_pct, vol = it
            noisy_avg = avg_ms * (0.85 + random.random() * 0.3)
            noisy_err = err_pct * (0.7 + random.random() * 0.6)
            noisy_vol = max(1, int(vol * (0.9 + random.random() * 0.2)))
            hist = generate_histogram(client, server, method, route,
                                      noisy_avg, min(noisy_err, 99), noisy_vol)
            lines.append(hist)

        body = "\n\n".join(lines) + "\n"

        self.send_response(200)
        self.send_header("Content-Type", "text/plain; version=0.0.4")
        self.end_headers()
        self.wfile.write(body.encode("utf-8"))

    def log_message(self, fmt, *args):
        pass


if __name__ == "__main__":
    server = http.server.HTTPServer((HOST, PORT), MetricsHandler)
    print(f"Serving eShopOnContainers metrics on http://{HOST}:{PORT}/metrics")
    server.serve_forever()
