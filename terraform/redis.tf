data "google_compute_zones" "zones" {
  region = local.location
}

data "google_compute_network" "default" {
  name = "default"
}

data "google_compute_subnetwork" "default" {
  name   = "default"
  region = data.google_compute_zones.zones.region
}

resource "google_service_account" "app_redis" {
  account_id = "${local.app}-redis"
}

resource "google_compute_disk" "redis_data" {
  name = "pd-redis-data"
  size = 10
  type = "pd-standard"
  zone = data.google_compute_zones.zones.names[0]
}

resource "google_compute_instance" "redis" {
  name         = "${local.app}-redis"
  machine_type = "f1-micro"
  zone         = google_compute_disk.redis_data.zone

  boot_disk {
    initialize_params {
      image = "cos-cloud/cos-stable"
      size  = 10
      type  = "pd-standard"
    }
  }

  attached_disk {
    mode        = "READ_WRITE"
    device_name = google_compute_disk.redis_data.name
    source      = google_compute_disk.redis_data.self_link
  }

  metadata = {
    google-logging-enabled    = "true"
    gce-container-declaration = <<EOT
    spec:
      containers:
        - name: "${local.app}-redis"
          image: redis:6.0
          command:
            - redis-server
          args:
            - '--appendonly'
            - 'yes'
          stdin: false
          tty: false
          volumeMounts:
            - name: data
              mountPath: /data
              readOnly: false
      restartPolicy: Always
      volumes:
          - name: data
            gcePersistentDisk:
              pdName: ${google_compute_disk.redis_data.name}
              fsType: ext4
              readOnly: false

    EOT
  }

  network_interface {
    network    = data.google_compute_network.default.self_link
    subnetwork = data.google_compute_subnetwork.default.self_link

    // we need either a nat or a public ip, since we need to pull public docker images
    access_config {
      network_tier = "STANDARD"
    }
  }

  service_account {
    email = google_service_account.app_redis.email
    scopes = [
      "https://www.googleapis.com/auth/cloud-platform",
    ]
  }
}