resource "google_cloud_run_service" "app" {
  name     = "utrakr-api"
  location = "us-west1"

  template {
    metadata {
      annotations = {
        "autoscaling.knative.dev/maxScale" = "10"
        "run.googleapis.com/client-name"   = "terraform"
      }
    }

    spec {
      container_concurrency = 80
      service_account_name  = "575736837658-compute@developer.gserviceaccount.com"

      containers {
        image = "us.gcr.io/utrakr/utrakr-api:df7198b"

        env {
          name  = "HOMEPAGE"
          value = "https://www.utrakr.app/"
        }
        env {
          name = "DEFAULT_BASE_HOST"
          value = "utrakr.app"
        }
        env {
          name = "DEFAULT_SECURE_HOST"
          value = "true"
        }
        env {
          name = "REDIS_URLS_CLIENT_CONN"
          value = "redis://127.0.0.1"
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
