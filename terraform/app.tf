resource "google_service_account" "app" {
  account_id = local.app
}

// add roles for instance logs writing, and metrics
resource "google_project_iam_member" "logs" {
  member = "serviceAccount:${google_service_account.app.email}"
  role   = "roles/logging.logWriter"
}

// add roles for instance logs writing
resource "google_project_iam_member" "metrics" {
  member = "serviceAccount:${google_service_account.app.email}"
  role   = "roles/monitoring.metricWriter"
}

// gcr is just a bucket
resource "google_storage_bucket_iam_member" "gcr" {
  bucket = "us.artifacts.utrakr.appspot.com"
  member = "serviceAccount:${google_service_account.app.email}"
  role   = "roles/storage.objectViewer"
}

resource "google_storage_bucket" "app_data" {
  name = "utrakr-prod-utrakr-api-data"
}

resource "google_storage_bucket_iam_member" "app_event_logs_member" {
  bucket = google_storage_bucket.app_data.name
  member = "serviceAccount:${google_service_account.app.email}"
  role   = "roles/storage.admin"
}

data "google_compute_zones" "default" {
  region = data.google_compute_subnetwork.default.region
}

resource "random_shuffle" "app_zones" {
  input        = data.google_compute_zones.default.names
  result_count = 1
}

resource "google_compute_disk" "app_data" {
  for_each = toset(random_shuffle.app_zones.result)
  name     = "pd-${local.app}-data"
  type     = "pd-standard"
  size     = 10
  zone     = each.key
}

resource "google_compute_address" "app" {
  for_each     = google_compute_disk.app_data
  name         = "${local.app}-${each.key}"
  network_tier = "STANDARD"
}

data "template_file" "app_startup" {
  for_each = google_compute_disk.app_data
  template = file("${path.module}/startup.sh")
  vars = {
    device_name = each.value.name
    // https://www.freedesktop.org/software/systemd/man/systemd.unit.html#String%20Escaping%20for%20Inclusion%20in%20Unit%20Names
    // important do not use - which means folder
    device_folder = "app_data" // mounted to /mnt/disks/<device_folder>
  }
}

resource "google_compute_instance" "app" {
  for_each                  = google_compute_disk.app_data
  name                      = local.app
  machine_type              = "e2-micro"
  zone                      = each.value.zone
  allow_stopping_for_update = true

  boot_disk {
    initialize_params {
      image = "cos-cloud/cos-stable"
      size  = 10
      type  = "pd-standard"
    }
  }

  attached_disk {
    mode        = "READ_WRITE"
    device_name = each.value.name
    source      = each.value.self_link
  }

  metadata = {
    google-logging-enabled = "false" // with lots of logs this could cost a bunch of money
  }

  metadata_startup_script = data.template_file.app_startup[each.key].rendered

  tags = [
    "http-server",
    "https-server",
    "traefik-server",
  ]

  network_interface {
    network    = data.google_compute_network.default.self_link
    subnetwork = data.google_compute_subnetwork.default.self_link

    // we need either a nat or a public ip, since we need to pull public docker images
    access_config {
      nat_ip       = google_compute_address.app[each.key].address
      network_tier = google_compute_address.app[each.key].network_tier
    }
  }

  service_account {
    email = google_service_account.app.email
    scopes = [
      "https://www.googleapis.com/auth/cloud-platform",
    ]
  }
}

resource "google_dns_record_set" "apex" {
  managed_zone = data.google_dns_managed_zone.root.name
  name         = data.google_dns_managed_zone.root.dns_name

  type    = "A"
  rrdatas = [for _, v in google_compute_instance.app : v.network_interface[0].access_config[0].nat_ip]
  ttl     = 3600
}

resource "google_dns_record_set" "api" {
  managed_zone = data.google_dns_managed_zone.root.name
  name         = "api.${data.google_dns_managed_zone.root.dns_name}"

  type    = "CNAME"
  rrdatas = [google_dns_record_set.apex.name]
  ttl     = 3600
}
