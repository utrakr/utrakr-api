variable "app_version" {
  default = "caf71f3"
}

resource "google_service_account" "app" {
  account_id = local.app
}

resource "google_cloud_run_service" "app" {
  name     = local.app
  location = local.location

  template {
    metadata {
      annotations = {
        "autoscaling.knative.dev/maxScale" = "10"
        "run.googleapis.com/client-name"   = "terraform"
      }
    }

    spec {
      container_concurrency = 80
      service_account_name  = google_service_account.app.email

      containers {
        image = "us.gcr.io/utrakr/utrakr-api:${var.app_version}"

        env {
          name  = "REDIRECT_HOMEPAGE"
          value = data.terraform_remote_state.homepage.outputs["homepage"]
        }
        env {
          name  = "DEFAULT_BASE_HOST"
          value = "utrakr.app"
        }
        env {
          name  = "COOKIE_SECURE"
          value = "true"
        }
        env {
          name  = "REDIS_URLS_CLIENT_CONN"
          value = "redis://${google_compute_instance.redis.network_interface[0].network_ip}"
        }

        resources {
          limits = {
            "cpu"    = "1000m"
            "memory" = "128Mi"
          }
          requests = {}
        }
      }
    }
  }

  timeouts {

  }

  traffic {
    latest_revision = true
    percent         = 100
  }

  autogenerate_revision_name = true
}

resource "google_cloud_run_domain_mapping" "app" {
  name     = trimsuffix(data.google_dns_managed_zone.root.dns_name, ".")
  location = google_cloud_run_service.app.location

  metadata {
    namespace = google_service_account.app.project
  }

  spec {
    route_name = google_cloud_run_service.app.name
  }
}

resource "google_dns_record_set" "app_dns" {
  for_each = {
    for dns_type in toset([
      for zone in google_cloud_run_domain_mapping.app.status[0]["resource_records"] : zone.type
    ]) :
    dns_type => toset([
      for zone in google_cloud_run_domain_mapping.app.status[0]["resource_records"] : zone.rrdata if zone.type == dns_type
    ])
  }

  name         = data.google_dns_managed_zone.root.dns_name
  managed_zone = data.google_dns_managed_zone.root.name
  type         = each.key
  rrdatas      = tolist(each.value)
  ttl          = 3600
}
